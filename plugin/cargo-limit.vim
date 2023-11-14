" TODO: enable linter
" TODO: check if diff is somehow broken

const MIN_NVIM_VERSION = '0.7.0'
const s:EXPECTED_PROTOCOL_VERSION = '0.0.10'

let s:data_chunks = []
let s:locations = []

function! s:on_cargo_metadata(_job_id, data, event)
  if a:event == 'stdout'
    call add(s:data_chunks, join(a:data, ''))
  elseif a:event == 'stderr' && type(a:data) == v:t_list && a:data != ['']
    let l:stderr = join(a:data, "\n")
    if l:stderr !~ 'could not find `Cargo.toml`'
      call s:log_error(l:stderr)
    endif
  elseif a:event == 'exit'
    let l:stdout = join(s:data_chunks, '')
    if len(l:stdout) > 0
      let l:metadata = json_decode(l:stdout)
      let l:workspace_root = get(l:metadata, 'workspace_root')
      let l:escaped_workspace_root = s:escape_path(workspace_root)
      call s:start_server(l:escaped_workspace_root)
    endif
  endif
endfunction

function! s:start_server(escaped_workspace_root)
  const TEMP_DIR_PREFIX = 'nvim-cargo-limit-'
  if has('unix')
    let l:server_address = '/tmp/' . TEMP_DIR_PREFIX . $USER . '/' . a:escaped_workspace_root
    let s:sources_dir = l:server_address . '.sources'
    call mkdir(s:sources_dir, 'p', 0700)
    call s:maybe_delete_dead_unix_socket(l:server_address)
  elseif has('win32')
    let l:server_address = '\\.\pipe\' . TEMP_DIR_PREFIX . $USERNAME . '-' . a:escaped_workspace_root
    " TODO: create sources dir
  else
    throw 'unsupported OS'
  endif

  if !filereadable(l:server_address)
    call serverstart(l:server_address)
    call s:log_info('ready')
  endif
endfunction

" TODO: naming
function! s:on_buffer_write()
  function! s:parse_lines(text, delimiter)
    let l:offset_and_lines = split(split(a:text, a:delimiter)[0], ',')
    let l:offset = str2nr(l:offset_and_lines[0])
    let l:lines = len(l:offset_and_lines) > 1 ? str2nr(l:offset_and_lines[1]) : 1
    return [l:offset, l:lines]
  endfunction

  " FIXME: why current file? what if we switch tab?
  let l:current_file = s:current_file()
  if l:current_file != '' && !filereadable(l:current_file) " TODO: correct?
    return
  endif
  "call s:log_info(l:current_file)

  let l:locations_index = 0
  while l:locations_index < len(l:locations_index) - 1
    if s:locations[l:locations_index]['path'] == l:current_file
      break
    else
      let l:locations_index += 1
    endif
  endwhile
"  call s:log_info(l:locations_index)
"  call s:log_info(s:locations)

  const DIFF_CHANGE_PATTERN = '@@ '
  const DIFF_NEW_CHANGES_COMMAND =
    \ 'w !git diff --unified=0 --ignore-all-space --no-index --no-color --no-ext-diff -- '
    \ . fnameescape(s:temp_source_for_diff(l:current_file))
    \ . ' '
    \ . l:current_file
  "call s:log_info(DIFF_NEW_CHANGES_COMMAND)

  let l:changed_line_numbers = {}
  let l:diff_stdout_lines = split(execute(DIFF_NEW_CHANGES_COMMAND), "\n")
  "call s:log_info(join(l:diff_stdout_lines, ''))
  let l:diff_stdout_line_number = 0
  while l:diff_stdout_line_number < len(l:diff_stdout_lines) - 1
    let l:diff_line = l:diff_stdout_lines[l:diff_stdout_line_number]
    if s:starts_with(l:diff_line, DIFF_CHANGE_PATTERN)
      let l:offsets_and_changes = split(trim(split(l:diff_line, DIFF_CHANGE_PATTERN)[0]), ' ')

      let [l:removal_offset, l:removal_lines] = s:parse_lines(l:offsets_and_changes[0], '-')
      let [l:addition_offset, l:addition_lines] = s:parse_lines(l:offsets_and_changes[1], '+')
      let l:changed_lines = l:addition_lines - l:removal_lines
      "let l:changed_lines = (l:removal_offset - l:addition_offset) + l:addition_lines - l:removal_lines " TODO

      let l:next_diff_line = l:diff_stdout_lines[l:diff_stdout_line_number + 1] " FIXME: bounds check?
      let l:removed_text = l:next_diff_line[1:]
      let l:removed_new_line = empty(l:removed_text)
      if !l:removed_new_line
        let l:changed_line_numbers[l:removal_offset] = 1
      endif

      "call s:log_info(l:offsets_and_changes)

      "call s:log_info(l:changed_lines)
      if l:changed_lines != 0
        while l:locations_index < len(s:locations)
          let l:current_location = s:locations[l:locations_index]
          if l:current_location['path'] == l:current_file
            let l:current_line = l:current_location['line']
            if l:current_line > l:removal_offset " TODO: && l:current_line <= l:removal_offset + l:changed_lines
              let s:locations[l:locations_index]['line'] += l:changed_lines
            endif
            let l:locations_index += 1
          else
            break
          endif
        endwhile
      endif
    endif
    let l:diff_stdout_line_number += 1
  endwhile

  call s:ignore_changed_lines_of_current_file(l:changed_line_numbers, l:current_file)
endfunction

" FIXME: naming
function! s:open_all_locations_in_new_or_existing_tabs(locations)
  call s:recreate_sources_dir()

  let l:current_file = s:current_file()
  if l:current_file == '' || filereadable(l:current_file)
    let s:locations = reverse(a:locations)
    call s:deduplicate_locations_by_paths_and_lines()
    for location in s:locations
      let l:path = fnameescape(location.path)
      if mode() == 'n' && &l:modified == 0
        execute 'tab drop ' . l:path
        call cursor((location.line), (location.column))
        call s:maybe_copy_to_sources(l:path)
      else
        break
      endif
    endfor
    let s:locations = reverse(s:locations)[1:]
  endif
endfunction

function! s:open_next_location_in_new_or_existing_tab()
  let l:current_file = s:current_file()
  if l:current_file == '' || filereadable(l:current_file) && !empty(s:locations)
    let l:location = s:locations[0]
    let l:path = fnameescape(l:location.path)
    if &l:modified == 0
      execute 'tab drop ' . l:path
      call cursor((l:location.line), (l:location.column))
      "call s:maybe_copy_to_sources(l:path) " TODO
      let s:locations = s:locations[1:]
    endif
  endif
endfunction

function! s:ignore_changed_lines_of_current_file(changed_line_numbers, current_file)
  let l:new_locations = []
  for i in s:locations
    let l:is_changed_line = get(a:changed_line_numbers, i.line) && i.path == a:current_file
    if !l:is_changed_line
      call add(l:new_locations, i)
    endif
  endfor
  let s:locations = l:new_locations
endfunction

function! s:deduplicate_locations_by_paths_and_lines()
  let l:new_locations = []
  let l:added_lines = {}

  for i in s:locations
    let l:added_line_key = string([i.path, i.line])
    let l:is_added_line = get(l:added_lines, l:added_line_key)
    if !l:is_added_line
      call add(l:new_locations, i)
      let l:added_lines[l:added_line_key] = 1
    endif
  endfor

  let s:locations = l:new_locations
endfunction

function! s:maybe_delete_dead_unix_socket(server_address)
  const LSOF_EXECUTABLE = 'lsof'
  const LSOF_COMMAND = LSOF_EXECUTABLE . ' -U'
  if filereadable(a:server_address)
    call system('which ' . LSOF_EXECUTABLE)
    let l:lsof_is_installed = v:shell_error == 0
    if l:lsof_is_installed
      let l:lsof_stdout = system(LSOF_COMMAND)
      let l:lsof_succeed = v:shell_error == 0
      if l:lsof_succeed
        let l:socket_is_dead = stridx(l:lsof_stdout, a:server_address) == -1
        if l:socket_is_dead
          let l:ignore = luaeval('os.remove(_A)', a:server_address)
          call s:log_info('removed dead socket ' . a:server_address)
        endif
      else
        call s:log_error('failed to execute "' . LSOF_COMMAND . '"')
      endif
    endif
  endif
endfunction

function! s:recreate_sources_dir()
  if exists('s:sources_dir')
    call delete(s:sources_dir, 'rf')
    call mkdir(s:sources_dir, 'p')
  endif
endfunction

function! s:current_file()
  return resolve(expand('%:p'))
endfunction

function! s:escape_path(path)
  return substitute(a:path, '[/\\:]', '%', 'g')
endfunction

" TODO: naming
function! s:temp_source_for_diff(path)
  "return s:sources_dir . '/' . fnamemodify(a:path, ':t') " TODO
  return s:sources_dir . '/' . s:escape_path(a:path)
endfunction

function! s:maybe_copy_to_sources(path)
  call s:maybe_copy(a:path, s:temp_source_for_diff(a:path))
endfunction

function! s:maybe_copy(source, destination)
  const MAX_SIZE_BYTES = 1024 * 1024
  if getfsize(a:source) <= MAX_SIZE_BYTES
    let l:data = readblob(a:source)
    call writefile(l:data, a:destination, "bS")
  endif
endfunction

function! s:starts_with(longer, shorter)
  return a:longer[0 : len(a:shorter) - 1] ==# a:shorter
endfunction

function! s:log_error(message)
  echohl Error
  echon 'cargo-limit: ' . a:message
  echohl None
endfunction

function! s:log_info(message)
  echohl None
  echomsg 'cargo-limit: ' . a:message
endfunction

if !exists('*CargoLimitOpen')
  function! g:CargoLimitOpen(editor_data)
    if exists('a:editor_data.protocol_version')
      let l:version_matched = a:editor_data.protocol_version == s:EXPECTED_PROTOCOL_VERSION
    else
      let l:version_matched = 0
    endif

    if !l:version_matched
      throw 'version mismatch, please update both nvim plugin and crate'
    endif

    let l:locations = a:editor_data.files
    call s:open_all_locations_in_new_or_existing_tabs(l:locations)
  endfunction

  function! g:CargoLimitOpenNextLocation()
    "call s:on_buffer_write()
    call s:open_next_location_in_new_or_existing_tab()
  endfunction

  augroup CargoLimitAutocommands
    autocmd!
    autocmd BufWritePost *.rs call s:on_buffer_write()
    autocmd VimLeavePre * call s:recreate_sources_dir()
  augroup END
endif

if has('nvim')
  if !has('nvim-' . MIN_NVIM_VERSION)
    throw 'unsupported nvim version, expected >=' . MIN_NVIM_VERSION
  endif
  call jobstart(['cargo', 'metadata', '--quiet', '--format-version=1'], {
  \ 'on_stdout': function('s:on_cargo_metadata'),
  \ 'on_stderr': function('s:on_cargo_metadata'),
  \ 'on_exit': function('s:on_cargo_metadata'),
  \ })
else
  throw 'unsupported text editor'
endif

" vim:shiftwidth=2 softtabstop=2 tabstop=2
