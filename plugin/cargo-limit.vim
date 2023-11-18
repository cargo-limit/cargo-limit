" TODO: enable linter: https://github.com/Vimjas/vint + https://github.com/Vimjas/vint/issues/367
" TODO: check if diff is somehow broken
" FIXME: regression? jump should not happen while I'm editing a file

function! s:main()
  const MIN_NVIM_VERSION = '0.7.0'

  if has('nvim')
    if !has('nvim-' . MIN_NVIM_VERSION)
      throw 'unsupported nvim version, expected >=' . MIN_NVIM_VERSION
    endif

    let s:DATA_CHUNKS = []
    let s:LOCATIONS = []
    call jobstart(['cargo', 'metadata', '--quiet', '--format-version=1'], {
    \ 'on_stdout': function('s:on_cargo_metadata'),
    \ 'on_stderr': function('s:on_cargo_metadata'),
    \ 'on_exit': function('s:on_cargo_metadata'),
    \ })
  else
    throw 'unsupported text editor'
  endif
endfunction

function! s:on_cargo_metadata(_job_id, data, event)
  if a:event == 'stdout'
    call add(s:DATA_CHUNKS, join(a:data, ''))
  elseif a:event == 'stderr' && type(a:data) == v:t_list
    let l:stderr = trim(join(a:data, "\n"))
    "call s:log_info(a:event . ' ' . !empty(l:stderr) . ' ' . (l:stderr !~# 'could not find `Cargo.toml`') . ' ' . (!s:contains_str(l:stderr, 'could not find `Cargo.toml`')))
    "if !empty(l:stderr) && l:stderr !~# 'could not find `Cargo.toml`' " TODO
    if !empty(l:stderr) && !s:contains_str(l:stderr, 'could not find `Cargo.toml`')
      call s:log_error(l:stderr)
    endif
  elseif a:event == 'exit'
    let l:stdout = trim(join(s:DATA_CHUNKS, ''))
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

  " TODO: what happens when I change dir to other crate? or run :source $MYVIMRC?
  if has('unix')
    let l:server_address = '/tmp/' . TEMP_DIR_PREFIX . $USER . '/' . a:escaped_workspace_root
    let s:TEMP_SOURCES_DIR = l:server_address . SOURCES
    call s:maybe_delete_dead_unix_socket(l:server_address)
  elseif has('win32')
    const SERVER_ADDRESS_POSTFIX = TEMP_DIR_PREFIX . $USERNAME . '-' . a:escaped_workspace_root
    let l:server_address = '\\.\pipe\' . SERVER_ADDRESS_POSTFIX
    let s:TEMP_SOURCES_DIR = $TEMP . '\' . SERVER_ADDRESS_POSTFIX . SOURCES
  else
    throw 'unsupported OS'
  endif

  if !filereadable(l:server_address)
    call s:recreate_temp_sources_dir()
    call s:maybe_setup_handlers()
    call serverstart(l:server_address)
    call s:log_info('ready')
  endif
endfunction

function! s:maybe_setup_handlers()
  if exists('*CargoLimitOpen')
    return
  endif

  function! g:CargoLimitOpen(editor_data)
    const PLUGIN_VERSION = '0.0.10' " TODO: if we knew plugin full path, we could
                                    " cargo metadata --quiet --format-version=1 --manifest-path ../Cargo.toml | jq | grep 'cargo-limit '

    let l:crate_version = v:null
    let l:version_matched = 0
    if exists('a:editor_data.protocol_version')
      let l:crate_version = a:editor_data.protocol_version
      let l:version_matched = l:crate_version == PLUGIN_VERSION
    endif

    if !l:version_matched
      " NOTE: this will become error after next breaking protocol change
      " call s:log_error('version mismatch, plugin ' . PLUGIN_VERSION . ' != crate ' . l:crate_version)
    endif

    let l:locations = a:editor_data.files
    call s:open_all_locations_in_new_or_existing_tabs(l:locations)
  endfunction

  function! g:CargoLimitOpenNextLocation()
    echom ''
    call s:open_next_location_in_new_or_existing_tab()
  endfunction

  augroup CargoLimitAutocommands
    autocmd!
    autocmd VimLeavePre * call s:recreate_temp_sources_dir()
    autocmd BufWritePost *.rs call s:on_buffer_write()
  augroup END
endfunction

function! s:open_all_locations_in_new_or_existing_tabs(locations)
  call s:recreate_temp_sources_dir()

  let l:current_file = s:current_file()
  if l:current_file != '' && !filereadable(l:current_file)
    return
  endif

  let s:LOCATIONS = reverse(a:locations)
  call s:deduplicate_locations_by_paths_and_lines() " TODO

  let l:location_index = 0
  while l:location_index < len(s:LOCATIONS)
    if mode() == 'n' && &l:modified == 0
      let l:path = fnameescape(s:LOCATIONS[l:location_index].path)
      execute 'tab drop ' . l:path
      call s:jump_to_location(l:location_index)
      call s:maybe_copy_to_temp_sources(l:path)
    else
      break
    endif
    let l:location_index += 1
  endwhile

  let s:LOCATIONS = reverse(s:LOCATIONS)[1:]
endfunction

function! s:open_next_location_in_new_or_existing_tab()
  let l:current_file = s:current_file()
  if (l:current_file != '' && !filereadable(l:current_file)) || empty(s:LOCATIONS) " TODO: correct?
    return
  endif

  if &l:modified == 0
    let l:path = fnameescape(s:LOCATIONS[0].path)
    execute 'tab drop ' . l:path
    "call s:update_locations(l:path) " TODO
    call s:jump_to_location(0)
    "call s:maybe_copy_to_temp_sources(l:path) " TODO
    let s:LOCATIONS = s:LOCATIONS[1:]
  endif
endfunction

function! s:on_buffer_write()
  let l:current_file = s:current_file()
  if l:current_file != '' && filereadable(l:current_file)
    call s:update_locations(l:current_file)
  endif
endfunction

function! s:update_locations(path)
  "call s:log_info('update_locations ' . a:path . ' BEG locations = ' . json_encode(s:LOCATIONS))

  const DIFF_STATS_PATTERN = '@@ '
  const DIFF_COMMAND =
    \ 'w !git diff --unified=0 --ignore-all-space --no-index --no-color --no-ext-diff -- '
    \ . fnameescape(s:temp_source_path(a:path))
    \ . ' '
    \ . a:path
  "call s:log_info(DIFF_COMMAND)

  let l:line_to_shift = []
  let l:edited_line_numbers = {}
  let l:diff_stdout_lines = split(execute(DIFF_COMMAND), "\n")
  let l:diff_stdout_line_number = 0
  while l:diff_stdout_line_number < len(l:diff_stdout_lines) - 1
    let l:diff_line = l:diff_stdout_lines[l:diff_stdout_line_number]
    if s:starts_with(l:diff_line, DIFF_STATS_PATTERN)
      let l:raw_diff_stats = split(split(l:diff_line, DIFF_STATS_PATTERN)[0], ' ')

      let [l:removal_offset, l:removals] = s:parse_diff_stats(l:raw_diff_stats[0], '-')
      let [l:addition_offset, l:additions] = s:parse_diff_stats(l:raw_diff_stats[1], '+')
      let l:shifted_lines = l:additions - l:removals

      call add(l:line_to_shift, [l:removal_offset, l:shifted_lines])
      let l:edited_line_numbers = s:update_edited_line_numbers(l:edited_line_numbers, l:removal_offset, l:removals, l:diff_stdout_lines, l:diff_stdout_line_number)
    endif
    let l:diff_stdout_line_number += 1
  endwhile

  let l:shift_accumulator = 0
  let l:line_to_shift_index = 0
  while l:line_to_shift_index < len(l:line_to_shift)
    let l:shifted_lines = l:line_to_shift[l:line_to_shift_index][1]
    let l:start = l:line_to_shift[l:line_to_shift_index][0]
    let l:end = l:line_to_shift_index + 1 < len(l:line_to_shift) ? l:line_to_shift[l:line_to_shift_index + 1][0] : v:null



    let l:locations_index = 0
    while l:locations_index < len(s:LOCATIONS)
      let l:current_location = s:LOCATIONS[l:locations_index]
      if l:current_location.path == a:path
        "call s:log_info('current_line ' . l:current_line . ' >= ' . l:start . ' && (' . l:end . ' == v:null || ' . l:current_line . ' <= ' . l:end . ') = ' . (l:current_line >= l:start && (l:end == v:null || l:current_line <= l:end)))
        let l:current_line = l:current_location.line
        "if l:current_line > l:start && l:current_line <= l:end - l:prev_shift "+ l:shifted_lines
        if l:current_line >= l:start && (l:end == v:null || l:current_line <= l:end)
          let s:LOCATIONS[l:locations_index].line += l:shifted_lines + l:shift_accumulator
        endif
      endif
      let l:locations_index += 1
    endwhile

    let l:shift_accumulator += l:shifted_lines

    let l:line_to_shift_index += 1
  endwhile

  call s:deduplicate_locations_by_paths_and_lines() " TODO: why for all paths?
  call s:ignore_edited_lines_of_current_file(l:edited_line_numbers, a:path)
  " TODO: deduplicate_locations_by_paths_and_lines + ignore_edited_lines_of_current_file
endfunction

function! s:parse_diff_stats(text, delimiter)
  let l:offset_and_lines = split(split(a:text, a:delimiter)[0], ',')
  let l:offset = str2nr(l:offset_and_lines[0])
  let l:lines = len(l:offset_and_lines) > 1 ? str2nr(l:offset_and_lines[1]) : 1
  return [l:offset, l:lines]
endfunction

function! s:update_edited_line_numbers(edited_line_numbers, removal_offset, removals, diff_stdout_lines, diff_stdout_line_number)
  let l:i = 0
  while l:i < a:removals
    let l:next_diff_line = a:diff_stdout_lines[a:diff_stdout_line_number + l:i]
    let l:edited_new_line = empty(l:next_diff_line[1:])
    if !l:edited_new_line
      let a:edited_line_numbers[a:removal_offset + l:i] = 1
    endif
    let l:i += 1
  endwhile
  return a:edited_line_numbers
endfunction

" TODO: naming
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

function! s:maybe_delete_dead_unix_socket(server_address)
  const LSOF_EXECUTABLE = 'lsof'
  const LSOF_COMMAND = LSOF_EXECUTABLE . ' -U'

  if !filereadable(a:server_address)
    return
  endif

  call system('which ' . LSOF_EXECUTABLE)
  let l:lsof_is_installed = v:shell_error == 0
  if !l:lsof_is_installed
    return
  endif

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
endfunction

function! s:recreate_temp_sources_dir()
  if exists('s:TEMP_SOURCES_DIR')
    call delete(s:TEMP_SOURCES_DIR, 'rf')
    call mkdir(s:TEMP_SOURCES_DIR, 'p', 0700)
  endif
endfunction

function! s:temp_source_path(path)
  "return s:TEMP_SOURCES_DIR . '/' . fnamemodify(a:path, ':t') " TODO
  return s:TEMP_SOURCES_DIR . '/' . s:escape_path(a:path)
endfunction

function! s:maybe_copy_to_temp_sources(path)
  call s:maybe_copy(a:path, s:temp_source_path(a:path))
endfunction

function! s:maybe_copy(source, destination)
  const MAX_SIZE_BYTES = 1024 * 1024
  if getfsize(a:source) <= MAX_SIZE_BYTES
    let l:data = readblob(a:source)
    call writefile(l:data, a:destination, 'bS')
  endif
endfunction

function! s:current_file()
  return resolve(expand('%:p'))
endfunction

function! s:escape_path(path)
  return substitute(a:path, '[/\\:]', '%', 'g')
endfunction

function! s:starts_with(longer, shorter)
  return a:longer[0 : len(a:shorter) - 1] ==# a:shorter
endfunction

function! s:contains_str(text, pattern)
  return stridx(a:text, a:pattern) != -1
endfunction

function! s:jump_to_location(location_index)
  let l:location = s:LOCATIONS[a:location_index]
  call cursor((l:location.line), (l:location.column))
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

call s:main()

" vim:shiftwidth=2 softtabstop=2 tabstop=2
