" TODO: enable linter
" TODO: check if diff is somehow broken
" TODO: reorder functions

const s:MIN_NVIM_VERSION = '0.7.0'
const s:PLUGIN_VERSION = '0.0.10' " TODO: if we knew plugin full path, we could
                                  " cargo metadata --quiet --format-version=1 --manifest-path ../Cargo.toml | jq | grep 'cargo-limit '

let s:DATA_CHUNKS = []
let s:LOCATIONS = []

function! s:on_cargo_metadata(_job_id, data, event)
  if a:event == 'stdout'
    call add(s:DATA_CHUNKS, join(a:data, ''))
  elseif a:event == 'stderr' && type(a:data) == v:t_list
    let l:stderr = join(a:data, "\n")
    "if !empty(l:stderr) && !s:contains_str(l:stderr, 'could not find `Cargo.toml`') " TODO
    if !empty(l:stderr) && l:stderr !~# 'could not find `Cargo.toml`'
      call s:log_error(l:stderr)
    endif
  elseif a:event == 'exit'
    let l:stdout = join(s:DATA_CHUNKS, '')
    if !empty(l:stdout)
      let l:metadata = json_decode(l:stdout)
      let l:workspace_root = get(l:metadata, 'workspace_root')
      let l:escaped_workspace_root = s:escape_path(workspace_root)
      call s:start_server(l:escaped_workspace_root)
    endif
  endif
endfunction

function! s:start_server(escaped_workspace_root)
  const TEMP_DIR_PREFIX = 'nvim-cargo-limit-'
  const SOURCES = '.sources'

  " TODO: what happens when I change dir to other crate? or run source $VIMRC?
  if has('unix')
    let l:server_address = '/tmp/' . TEMP_DIR_PREFIX . $USER . '/' . a:escaped_workspace_root
    let s:SOURCES_TEMP_DIR = l:server_address . SOURCES
    call s:maybe_delete_dead_unix_socket(l:server_address)
  elseif has('win32')
    const SERVER_ADDRESS_POSTFIX = TEMP_DIR_PREFIX . $USERNAME . '-' . a:escaped_workspace_root
    let l:server_address = '\\.\pipe\' . SERVER_ADDRESS_POSTFIX
    let s:SOURCES_TEMP_DIR = $TEMP . '\' . SERVER_ADDRESS_POSTFIX . SOURCES
  else
    throw 'unsupported OS'
  endif

  if !filereadable(l:server_address)
    call s:recreate_sources_temp_dir()
    call s:maybe_setup_handlers()
    call serverstart(l:server_address)
    call s:log_info('ready')
  endif
endfunction

" TODO: naming
function! s:on_buffer_write()
  " resolve(bufname()) ? nope
  " FIXME: why current file? what if we switch tab?
  " this will return file per each currently written tab
  let l:current_file = s:current_file()
  if l:current_file != '' && !filereadable(l:current_file) " TODO: correct?
    return
  endif
  "call s:log_info(l:current_file)

  let l:locations_index = 0
  while l:locations_index < len(l:locations_index) - 1
    if s:LOCATIONS[l:locations_index].path == l:current_file
      break
    else
      let l:locations_index += 1
    endif
  endwhile
"  call s:log_info(l:locations_index)
"  call s:log_info(s:LOCATIONS)

  const DIFF_STATS_PATTERN = '@@ '
  const DIFF_COMMAND =
    \ 'w !git diff --unified=0 --ignore-all-space --no-index --no-color --no-ext-diff -- '
    \ . fnameescape(s:temp_source_for_diff(l:current_file))
    \ . ' '
    \ . l:current_file
  "call s:log_info(DIFF_COMMAND)

  let l:edited_line_numbers = {}
  let l:diff_stdout_lines = split(execute(DIFF_COMMAND), "\n")
  "call s:log_info(join(l:diff_stdout_lines, ''))
  let l:diff_stdout_line_number = 0
  while l:diff_stdout_line_number < len(l:diff_stdout_lines) - 1
    let l:diff_line = l:diff_stdout_lines[l:diff_stdout_line_number]
    if s:starts_with(l:diff_line, DIFF_STATS_PATTERN)
      let l:raw_diff_stats = split(trim(split(l:diff_line, DIFF_STATS_PATTERN)[0]), ' ')

      let [l:removal_offset, l:removals] = s:parse_diff_stats(l:raw_diff_stats[0], '-')
      let [l:addition_offset, l:additions] = s:parse_diff_stats(l:raw_diff_stats[1], '+')
      let l:shifted_lines = l:additions - l:removals
      "let l:shifted_lines = (l:removal_offset - l:addition_offset) + l:additions - l:removals " TODO

      let l:next_diff_line = l:diff_stdout_lines[l:diff_stdout_line_number + 1]
      let l:edited_new_line = empty(l:next_diff_line[1:])
      if !l:edited_new_line
        let l:edited_line_numbers[l:removal_offset] = 1
      endif

      "call s:log_info(l:raw_diff_stats)

      "call s:log_info(l:shifted_lines)
      if l:shifted_lines != 0
        while l:locations_index < len(s:LOCATIONS)
          let l:current_location = s:LOCATIONS[l:locations_index]
          if l:current_location.path == l:current_file
            let l:current_line = l:current_location.line
            if l:current_line > l:removal_offset " TODO: && l:current_line <= l:removal_offset + l:shifted_lines
              let s:LOCATIONS[l:locations_index].line += l:shifted_lines
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

  call s:ignore_edited_lines_of_current_file(l:edited_line_numbers, l:current_file)
endfunction

function! s:parse_diff_stats(text, delimiter)
  let l:offset_and_lines = split(split(a:text, a:delimiter)[0], ',')
  let l:offset = str2nr(l:offset_and_lines[0])
  let l:lines = len(l:offset_and_lines) > 1 ? str2nr(l:offset_and_lines[1]) : 1
  return [l:offset, l:lines]
endfunction

" FIXME: naming
function! s:open_all_locations_in_new_or_existing_tabs(locations)
  call s:recreate_sources_temp_dir()

  let l:current_file = s:current_file()
  if l:current_file == '' || filereadable(l:current_file)
    let s:LOCATIONS = reverse(a:locations)
    call s:deduplicate_locations_by_paths_and_lines()
    for location in s:LOCATIONS
      let l:path = fnameescape(location.path)
      if mode() == 'n' && &l:modified == 0
        execute 'tab drop ' . l:path
        "call s:on_buffer_write()
        call cursor((location.line), (location.column))
        call s:maybe_copy_to_sources(l:path)
      else
        break
      endif
    endfor
    let s:LOCATIONS = reverse(s:LOCATIONS)[1:]
  endif
endfunction

function! s:open_next_location_in_new_or_existing_tab()
  let l:current_file = s:current_file()
  if l:current_file == '' || filereadable(l:current_file) && !empty(s:LOCATIONS)
    let l:location = s:LOCATIONS[0]
    let l:path = fnameescape(l:location.path)
    if &l:modified == 0
      execute 'tab drop ' . l:path
      "call s:on_buffer_write()
      call cursor((l:location.line), (l:location.column))
      "call s:maybe_copy_to_sources(l:path) " TODO
      let s:LOCATIONS = s:LOCATIONS[1:]
    endif
  endif
endfunction

function! s:ignore_edited_lines_of_current_file(edited_line_numbers, current_file)
  let l:new_locations = []
  for i in s:LOCATIONS
    let l:is_edited_line = get(a:edited_line_numbers, i.line) && i.path == a:current_file
    if !l:is_edited_line
      call add(l:new_locations, i)
    endif
  endfor
  let s:LOCATIONS = l:new_locations
endfunction

function! s:deduplicate_locations_by_paths_and_lines()
  let l:new_locations = []
  let l:added_lines = {}

  for i in s:LOCATIONS
    let l:added_line_key = string([i.path, i.line])
    let l:is_added_line = get(l:added_lines, l:added_line_key)
    if !l:is_added_line
      call add(l:new_locations, i)
      let l:added_lines[l:added_line_key] = 1
    endif
  endfor

  let s:LOCATIONS = l:new_locations
endfunction

function! s:maybe_setup_handlers()
  if !exists('*CargoLimitOpen')
    function! g:CargoLimitOpen(editor_data)
      if exists('a:editor_data.protocol_version')
        let l:crate_version = a:editor_data.protocol_version
        let l:version_matched = l:crate_version == s:PLUGIN_VERSION
      else
        let l:version_matched = 0
      endif

      if !l:version_matched
        " NOTE: this will become error after next breaking protocol change
        " call s:log_error('version mismatch, plugin ' . s:PLUGIN_VERSION . ' != crate ' . l:crate_version)
      endif

      let l:locations = a:editor_data.files
      call s:open_all_locations_in_new_or_existing_tabs(l:locations)
    endfunction

    function! g:CargoLimitOpenNextLocation()
      echom ''
      "call s:on_buffer_write()
      call s:open_next_location_in_new_or_existing_tab()
    endfunction

    augroup CargoLimitAutocommands
      autocmd!
      autocmd BufWritePost *.rs call s:on_buffer_write()
      autocmd VimLeavePre * call s:recreate_sources_temp_dir()
    augroup END
  endif
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
        let l:socket_is_dead = !s:contains_str(l:lsof_stdout, a:server_address)
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

function! s:recreate_sources_temp_dir()
  if exists('s:SOURCES_TEMP_DIR')
    call delete(s:SOURCES_TEMP_DIR, 'rf')
    call mkdir(s:SOURCES_TEMP_DIR, 'p', 0700)
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
  "return s:SOURCES_TEMP_DIR . '/' . fnamemodify(a:path, ':t') " TODO
  return s:SOURCES_TEMP_DIR . '/' . s:escape_path(a:path)
endfunction

function! s:maybe_copy_to_sources(path)
  call s:maybe_copy(a:path, s:temp_source_for_diff(a:path))
endfunction

function! s:maybe_copy(source, destination)
  const MAX_SIZE_BYTES = 1024 * 1024
  if getfsize(a:source) <= MAX_SIZE_BYTES
    let l:data = readblob(a:source)
    call writefile(l:data, a:destination, 'bS')
  endif
endfunction

function! s:starts_with(longer, shorter)
  return a:longer[0 : len(a:shorter) - 1] ==# a:shorter
endfunction

function! s:contains_str(text, pattern)
  return stridx(a:text, a:pattern) != -1
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

if has('nvim')
  if !has('nvim-' . s:MIN_NVIM_VERSION)
    throw 'unsupported nvim version, expected >=' . s:MIN_NVIM_VERSION
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
