" TODO: enable linter: https://github.com/Vimjas/vint + https://github.com/Vimjas/vint/issues/367
" TODO: check if diff is somehow broken?
" FIXME: regression? jump should not happen while I'm editing a file
" TODO: should we have CargoLimitOpenInternal (ccff595)?
"       that makes precise jumps possible for custom functions
"         also plugin version validation can be forced for everyone
"           instead we could check plugin version from rust code
"       or just call another function with corrected EDITOR_DATA?
"       or the same g:CargoLimitOpen with corrected flag argument/field?
"         or even location index? this one is probably unusable for custom functions

function! s:main()
  const MIN_NVIM_VERSION = '0.7.0'

  if has('nvim')
    if !has('nvim-' . MIN_NVIM_VERSION)
      throw 'unsupported nvim version, expected >=' . MIN_NVIM_VERSION
    endif

    let s:data_chunks = []
    let s:editor_data = {'locations': []}
    let s:location_index = v:null
    let s:edited_locations = {}
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
  if a:event ==# 'stdout'
    call add(s:data_chunks, join(a:data, ''))
  elseif a:event ==# 'stderr' && type(a:data) ==# v:t_list
    let l:stderr = trim(join(a:data, "\n"))
    "call s:log_info(a:event . ' ' . !empty(l:stderr) . ' ' . (l:stderr !~# 'could not find `Cargo.toml`') . ' ' . (!s:contains_str(l:stderr, 'could not find `Cargo.toml`')))
    "if !empty(l:stderr) && l:stderr !~# 'could not find `Cargo.toml`' " TODO
    if !empty(l:stderr) && !s:contains_str(l:stderr, 'could not find `Cargo.toml`')
      call s:log_error('cargo metadata failed', l:stderr, !empty(l:stderr), !s:contains_str(l:stderr, 'could not find `Cargo.toml`'), len(l:stderr), l:stderr !~# 'could not find `Cargo.toml`')
    endif
  elseif a:event ==# 'exit'
    let l:stdout = trim(join(s:data_chunks, ''))
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

  if has('unix')
    let l:server_address = '/tmp/' . TEMP_DIR_PREFIX . $USER . '/' . a:escaped_workspace_root
    let s:temp_sources_dir = l:server_address . SOURCES
    call s:maybe_delete_dead_unix_socket(l:server_address)
  elseif has('win32')
    const SERVER_ADDRESS_POSTFIX = TEMP_DIR_PREFIX . $USERNAME . '-' . a:escaped_workspace_root
    let l:server_address = '\\.\pipe\' . SERVER_ADDRESS_POSTFIX
    let s:temp_sources_dir = $TEMP . '\' . SERVER_ADDRESS_POSTFIX . SOURCES
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

function s:validate_plugin_version(editor_data)
  const PLUGIN_VERSION = '0.0.11'

  let l:crate_version = v:null
  let l:version_matched = v:false
  let l:crate_message_postfix = ' > crate version'
  if exists('a:editor_data.protocol_version')
    let l:crate_version = a:editor_data.protocol_version
    let l:version_matched = l:crate_version == PLUGIN_VERSION
    let l:crate_message_postfix = ' != crate ' . l:crate_version
  endif

  if !l:version_matched
    call s:log_error('version mismatch, plugin ' . PLUGIN_VERSION . l:crate_message_postfix)
  endif
endfunction

function! s:maybe_setup_handlers()
  augroup CargoLimitAutocommands
    autocmd!
    autocmd VimLeavePre * call s:recreate_temp_sources_dir()
    autocmd BufWritePost *.rs call s:on_buffer_write()
  augroup END

  if exists('*CargoLimitOpen')
    return
  endif

  function! g:CargoLimitOpen(editor_data)
    call s:validate_plugin_version(a:editor_data)

    let s:editor_data = a:editor_data
    let s:location_index = -1
    let s:edited_locations = {}

    if exists('s:editor_data.files')
      let s:editor_data.locations = s:editor_data.files
      call remove(s:editor_data, 'files')
    endif

    let l:current_file = s:current_file()
    if (l:current_file !=# '' && !filereadable(l:current_file)) || empty(s:editor_data.locations)
      return
    endif

    call s:open_all_locations_in_reverse_deduplicated_by_paths()
    call s:update_next_unique_location_index()
  endfunction

  function! g:CargoLimitOpenNextLocation()
    echomsg ''
    call s:open_next_location_in_new_or_existing_tab()
  endfunction

  function! g:CargoLimitOpenPrevLocation()
    echomsg ''
    call s:open_prev_location_in_new_or_existing_tab()
  endfunction
endfunction

function! s:open_all_locations_in_reverse_deduplicated_by_paths()
  call s:recreate_temp_sources_dir()

  let l:path_to_location_index = {}
  for i in range(len(s:editor_data.locations) - 1, 0, -1)
    let l:path_to_location_index[s:editor_data.locations[i].path] = i
  endfor

  for i in range(len(s:editor_data.locations) - 1, 0, -1)
    let l:path = s:editor_data.locations[i].path
    if !has_key(l:path_to_location_index, l:path)
      continue
    elseif mode() ==# 'n' && &l:modified ==# 0
      let l:location_index = l:path_to_location_index[l:path]
      call remove(l:path_to_location_index, l:path)

      " FIXME: shadow
      let l:path = fnameescape(s:editor_data.locations[i].path)
      call s:jump_to_location(l:location_index)
      call s:maybe_copy_to_temp_sources(l:path)
    else
      break
    endif
  endfor
endfunction

" TODO: don't extract?
function! s:open_next_location_in_new_or_existing_tab()
  let l:current_file = s:current_file()
  " TODO: &l:modified !=# 0 - is it correct here?
  if s:location_index !=# v:null && (empty(s:editor_data.locations) || s:location_index >=# len(s:editor_data.locations) || &l:modified !=# 0 || (l:current_file !=# '' && !filereadable(l:current_file)))
    return
  endif

  let l:initial_location_index = s:location_index

  call s:update_next_unique_location_index()

  if l:initial_location_index !=# s:location_index
    call s:jump_to_location(s:location_index)
  endif
endfunction

" TODO: don't extract?
function! s:open_prev_location_in_new_or_existing_tab()
  let l:current_file = s:current_file()
  " TODO: &l:modified !=# 0 - is it correct here?
  if s:location_index !=# v:null && (empty(s:editor_data.locations) || s:location_index <=# 0 || &l:modified !=# 0 || (l:current_file !=# '' && !filereadable(l:current_file)))
    return
  endif

  let l:initial_location_index = s:location_index

  call s:update_prev_unique_location_index()

  if l:initial_location_index !=# s:location_index
    call s:jump_to_location(s:location_index)
  endif
endfunction

" TODO: naming? refactoring?
function! s:update_next_unique_location_index()
  " go to next unedited location with different path or line
  let l:location = s:current_location()
  while s:location_index <# len(s:editor_data.locations) - 1 && (s:is_same_location(l:location, s:current_location()) || s:is_edited_location(s:current_location()))
    let s:location_index += 1
  endwhile

  " go to last unedited location on the same line
  while s:location_index <# len(s:editor_data.locations) - 1 && s:is_same_location(s:current_location(), s:next_location())
    let s:location_index += 1
  endwhile

  while s:location_index <# len(s:editor_data.locations) - 1 && s:is_edited_location(s:current_location())
    let s:location_index += 1
  endwhile

  while s:location_index <# len(s:editor_data.locations) - 1 && s:is_same_location(s:current_location(), s:next_location())
    let s:location_index += 1
  endwhile
endfunction

" TODO: naming? remove? refactoring?
function! s:update_prev_unique_location_index()
  let l:location = s:current_location()
  while s:location_index >=# 1 && (s:is_same_location(s:current_location(), l:location) || s:is_edited_location(s:current_location()))
    let s:location_index -= 1
  endwhile
endfunction

function! s:on_buffer_write()
  " TODO: can this function too early, before rustfmt finish?
  " TODO: run git diff with jobstart? or rerun it later?
  let l:current_file = s:current_file()
  if l:current_file !=# '' && filereadable(l:current_file)
    call s:update_locations(l:current_file)
    call s:maybe_copy_to_temp_sources(l:current_file)
    if exists('*CargoLimitUpdate')
      call g:CargoLimitUpdate(s:editor_data)
    endif
  endif
endfunction

function! s:update_locations(path)
  "call s:log_info('update_locations ' . a:path . ' BEG locations = ' . json_encode(s:editor_data.locations))

  let [l:line_to_shift, l:edited_line_numbers] = s:compute_shifts_and_edits(a:path)

  let l:shift_accumulator = 0
  for i in range(0, len(l:line_to_shift) - 1)
    let l:shifted_lines = l:line_to_shift[i][1]
    let l:start = l:line_to_shift[i][0]
    let l:end = i + 1 <# len(l:line_to_shift) ? l:line_to_shift[i + 1][0] : v:null
    let l:shift_accumulator += l:shifted_lines
    let l:edited_line_numbers = s:shift_locations(a:path, l:edited_line_numbers, l:start, l:end, l:shift_accumulator)
  endfor

  call s:ignore_edited_lines_of_current_file(l:edited_line_numbers, a:path)
endfunction

function! s:compute_shifts_and_edits(path)
  let l:temp_source_path = s:temp_source_path(a:path)

  const DIFF_STATS_PATTERN = '@@ '
  const DIFF_COMMAND =
    \ 'w !git diff --unified=0 --ignore-blank-lines --ignore-all-space --ignore-cr-at-eol --no-index --no-color --no-ext-diff -- '
    \ . fnameescape(l:temp_source_path)
    \ . ' '
    \ . a:path
  "call s:log_info(DIFF_COMMAND)

  let l:line_to_shift = [] " TODO: naming
  let l:edited_line_numbers = {}
  if !filereadable(l:temp_source_path)
    return [l:line_to_shift, l:edited_line_numbers]
  endif

  let l:diff_stdout_lines = split(execute(DIFF_COMMAND), "\n")
  let l:diff_stdout_line_number = 0 " TODO: rename to index?
  while l:diff_stdout_line_number <# len(l:diff_stdout_lines) - 1
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

  "call s:log_info(l:line_to_shift)
  return [l:line_to_shift, l:edited_line_numbers]
endfunction

function! s:shift_locations(path, edited_line_numbers, start, end, shift_accumulator)
"  let l:wat_lines = []
"  for i in s:editor_data.locations
"    call add(l:wat_lines, i.line)
"  endfor
"  call s:log_info('BEG lines', l:wat_lines)

  for i in range(0, len(s:editor_data.locations) - 1)
    let l:current_location = s:editor_data.locations[i] " TODO: why current? naming
    if l:current_location.path ==# a:path
      let l:current_line = l:current_location.line
      if l:current_line ># a:start && (a:end ==# v:null || l:current_line <# a:end)
        let s:editor_data.locations[i].line += a:shift_accumulator
      endif
    endif
  endfor


"  let l:wat_lines = []
"  for i in s:editor_data.locations
"    call add(l:wat_lines, i.line)
"  endfor
"  call s:log_info('END lines', l:wat_lines)

  " TODO
  for line in keys(a:edited_line_numbers)
    if line ># a:start && (a:end ==# v:null || line <# a:end)
      call remove(a:edited_line_numbers, line)
      let a:edited_line_numbers[line + a:shift_accumulator] = v:true
    endif
  endfor

  return a:edited_line_numbers
endfunction

function! s:parse_diff_stats(text, delimiter)
  let l:offset_and_lines = split(split(a:text, a:delimiter)[0], ',')
  let l:offset = str2nr(l:offset_and_lines[0])
  let l:lines = len(l:offset_and_lines) ># 1 ? str2nr(l:offset_and_lines[1]) : 1
  return [l:offset, l:lines]
endfunction

function! s:update_edited_line_numbers(edited_line_numbers, removal_offset, removals, diff_stdout_lines, diff_stdout_line_number)
  for i in range(0, a:removals - 1)
    let l:next_diff_line = a:diff_stdout_lines[a:diff_stdout_line_number + i]
    let a:edited_line_numbers[a:removal_offset + i] = v:true
  endfor
  return a:edited_line_numbers
endfunction

" TODO: naming
function! s:ignore_edited_lines_of_current_file(edited_line_numbers, current_file)
  for line in keys(a:edited_line_numbers)
    if !has_key(s:edited_locations, a:current_file)
      let s:edited_locations[a:current_file] = {}
    endif
    let s:edited_locations[a:current_file][line] = v:true
  endfor
endfunction

function! s:is_edited_location(location)
  return has_key(s:edited_locations, a:location.path) && has_key(s:edited_locations[a:location.path], a:location.line)
endfunction

function! s:is_same_location(first, second)
  return a:first.path ==# a:second.path && a:first.line ==# a:second.line
endfunction

function! s:maybe_delete_dead_unix_socket(server_address)
  const LSOF_EXECUTABLE = 'lsof'
  const LSOF_COMMAND = LSOF_EXECUTABLE . ' -U'

  if !filereadable(a:server_address)
    return
  endif

  call system('which ' . LSOF_EXECUTABLE)
  let l:lsof_is_installed = v:shell_error ==# 0
  if !l:lsof_is_installed
    return
  endif

  let l:lsof_stdout = system(LSOF_COMMAND)
  let l:lsof_succeed = v:shell_error ==# 0
  if l:lsof_succeed
    let l:socket_is_dead = !s:contains_str(l:lsof_stdout, a:server_address)
    if l:socket_is_dead
      let l:ignore = luaeval('os.remove(_A)', a:server_address)
      call s:log_info('removed dead socket', a:server_address)
    endif
  else
    call s:log_error('failed to execute', LSOF_COMMAND)
  endif
endfunction

function! s:recreate_temp_sources_dir()
  if exists('s:temp_sources_dir')
    call delete(s:temp_sources_dir, 'rf')
    call mkdir(s:temp_sources_dir, 'p', 0700)
  endif
endfunction

function! s:temp_source_path(path)
  "return s:temp_sources_dir . '/' . fnamemodify(a:path, ':t') " TODO
  return s:temp_sources_dir . '/' . s:escape_path(a:path)
endfunction

function! s:maybe_copy_to_temp_sources(path)
  call s:maybe_copy(a:path, s:temp_source_path(a:path))
endfunction

function! s:maybe_copy(source, destination)
  const MAX_SIZE_BYTES = 1024 * 1024
  if getfsize(a:source) <=# MAX_SIZE_BYTES
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
  return stridx(a:text, a:pattern) !=# -1
endfunction

function! s:jump_to_location(location_index)
  let l:location = s:editor_data.locations[a:location_index]
  " TODO: is fnameescape required here?
  execute 'tab drop ' . fnameescape(l:location.path)
  call cursor((l:location.line), (l:location.column))
endfunction

function! s:current_location()
  return s:editor_data.locations[s:location_index]
endfunction

function! s:next_location()
  return s:editor_data.locations[s:location_index + 1]
endfunction

function! s:prev_location()
  return s:editor_data.locations[s:location_index - 1]
endfunction

function! s:log_error(...)
  echohl Error
  redraw
  "echom s:log_str(a:000)
  echon s:log_str(a:000)
  echohl None
endfunction

function! s:log_info(...)
  echohl None
  redraw
  echon s:log_str(a:000)
endfunction

function! s:log_str(args)
  return '[cargo-limit] ' . join(a:args, ' ')
endfunction

call s:main()

" vim:shiftwidth=2 softtabstop=2 tabstop=2
