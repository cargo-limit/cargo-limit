fun! s:main() abort
  const MIN_NVIM_VERSION = '0.7.0'

  if has('nvim')
    if !has('nvim-' . MIN_NVIM_VERSION)
      throw 'unsupported nvim version, expected >=' . MIN_NVIM_VERSION
    endif

    if !exists('g:CargoLimitVerbosity')
      let g:CargoLimitVerbosity = 3 " info level
    endif
    let s:data_chunks = []
    let s:editor_data = {'locations': []}
    let s:locations_texts = {}
    let s:workspace_root = v:null
    let s:location_index = v:null
    let s:temp_sources_dir = v:null
    let s:deprecated_cargo_limit_open = v:null
    call jobstart(['cargo', 'metadata', '--quiet', '--format-version=1'], {
    \ 'on_stdout': function('s:on_cargo_metadata'),
    \ 'on_stderr': function('s:on_cargo_metadata'),
    \ 'on_exit': function('s:on_cargo_metadata'),
    \ })
  else
    throw 'unsupported text editor'
  endif
endf

fun! s:on_cargo_metadata(_job_id, data, event) abort
  if a:event ==# 'stdout'
    call add(s:data_chunks, join(a:data, ''))
  elseif a:event ==# 'stderr' && type(a:data) ==# v:t_list
    let l:stderr = trim(join(a:data, "\n"))
    if !empty(l:stderr) && !s:contains_str(l:stderr, 'could not find `Cargo.toml`')
      call s:log_error('cargo metadata', l:stderr)
    endif
  elseif a:event ==# 'exit'
    let l:stdout = trim(join(s:data_chunks, ''))
    if !empty(l:stdout)
      let l:metadata = json_decode(l:stdout)
      let s:workspace_root = get(l:metadata, 'workspace_root')
      let l:escaped_workspace_root = s:escape_path(s:workspace_root)
      call s:start_server(l:escaped_workspace_root)
    endif
  endif
endf

fun! s:start_server(escaped_workspace_root) abort
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
endf

fun! s:maybe_setup_handlers() abort
  augroup CargoLimitAutocommands
    autocmd!
    autocmd VimLeavePre * call s:recreate_temp_sources_dir()
    autocmd BufWritePost *.rs call s:on_buffer_write()
  augroup END

  if exists('*CargoLimitOpen')
    let s:deprecated_cargo_limit_open = funcref('g:CargoLimitOpen')
    call s:log_warn('g:CargoLimitOpen is deprecated, please migrate to g:CargoLimitUpdate: https://github.com/cargo-limit/cargo-limit#text-editoride-integrations')
  endif

  fun! g:CargoLimitOpen(editor_data) abort
    let s:editor_data = a:editor_data
    let s:locations_texts = {}
    let s:location_index = -1

    if s:deprecated_cargo_limit_open !=# v:null
      call s:downgrade_editor_data_format()
      call s:deprecated_cargo_limit_open(s:editor_data)
    endif

    call s:upgrade_editor_data_format()

    if s:deprecated_cargo_limit_open !=# v:null
      call s:increment_location_index()
      return
    endif

    call s:copy_affected_files_to_temp()

    if !exists('*CargoLimitUpdate')
      fun! g:CargoLimitUpdate(editor_data) abort
        let l:current_file = s:current_file()
        if (l:current_file !=# '' && !filereadable(l:current_file)) || empty(s:editor_data.locations)
          return
        endif

        if !a:editor_data.corrected_locations
          call s:deduplicate_locations_by_paths_and_lines()
          call s:open_all_locations_in_reverse()
          call s:read_all_locations_texts()
          call s:increment_location_index()
        endif
      endf
    endif

    let s:editor_data.corrected_locations = v:false
    call g:CargoLimitUpdate(s:editor_data)
  endf

  fun! g:CargoLimitOpenNextLocation() abort
    echomsg ''
    if empty(s:editor_data.locations)
      return
    endif

    let l:current_file = s:current_file()
    if &l:modified !=# 0 || (l:current_file !=# '' && !filereadable(l:current_file))
      return
    endif

    let l:initial_location_index = s:location_index
    call s:increment_location_index()
    if s:is_current_location_edited()
      let s:location_index = l:initial_location_index
    else
      call s:jump_to_location(s:location_index)
    endif
  endf

  fun! g:CargoLimitOpenPrevLocation() abort
    echomsg ''
    if empty(s:editor_data.locations)
      return
    endif

    let l:current_file = s:current_file()
    if &l:modified !=# 0 || (l:current_file !=# '' && !filereadable(l:current_file))
      return
    endif

    let l:initial_location_index = s:location_index
    call s:decrement_location_index()
    if s:is_current_location_edited()
      let s:location_index = l:initial_location_index
    else
      call s:jump_to_location(s:location_index)
    endif
  endf
endf

fun! s:downgrade_editor_data_format() abort
  if exists('s:editor_data.locations')
    let s:editor_data.files = s:editor_data.locations
    call remove(s:editor_data, 'locations')
    call remove(s:editor_data, 'corrected_locations')
  endif
  if !exists('s:editor_data.files')
    let s:editor_data.files = []
  endif
endf

fun! s:upgrade_editor_data_format() abort
  if exists('s:editor_data.files')
    let s:editor_data.locations = reverse(s:editor_data.files)
    call remove(s:editor_data, 'files')
  endif
  if !exists('s:editor_data.locations')
    let s:editor_data.locations = []
  endif
  if exists('s:editor_data.corrected_locations')
    let s:editor_data.corrected_locations = s:editor_data.corrected_locations ? v:true : v:false
  else
    let s:editor_data.corrected_locations = v:false
  endif
endf

fun! s:copy_affected_files_to_temp() abort
  call s:recreate_temp_sources_dir()

  let l:paths = {}
  for l:index in range(0, len(s:editor_data.locations) - 1)
    let l:paths[s:editor_data.locations[l:index].path] = v:true
  endfor

  for l:path in keys(l:paths)
    call s:maybe_copy_to_temp(l:path)
  endfor
endf

fun! s:open_all_locations_in_reverse() abort
  let l:path_to_location_index = {}
  for l:index in range(len(s:editor_data.locations) - 1, 0, -1)
    let l:path_to_location_index[s:editor_data.locations[l:index].path] = l:index
  endfor

  redraw!

  for l:index in range(len(s:editor_data.locations) - 1, 0, -1)
    let l:path = s:editor_data.locations[l:index].path
    if !has_key(l:path_to_location_index, l:path)
      continue
    elseif mode() ==# 'n' && &l:modified ==# 0
      let l:location_index = l:path_to_location_index[l:path]
      call remove(l:path_to_location_index, l:path)
      call s:jump_to_location(l:location_index)
    else
      break
    endif
  endfor

  redraw
endf

fun! s:increment_location_index() abort
  let l:initial_location = s:current_location()
  while s:location_index <# len(s:editor_data.locations) - 1
    let s:location_index += 1
    if s:is_same_as_current_location(l:initial_location)
      continue
    endif
    if !s:is_current_location_edited()
      break
    endif
  endwhile
endf

fun! s:decrement_location_index() abort
  let l:initial_location = s:current_location()
  while s:location_index >=# 1
    let s:location_index -= 1
    if s:is_same_as_current_location(l:initial_location)
      continue
    endif
    if !s:is_current_location_edited()
      break
    endif
  endwhile
endf

fun! s:update_locations(path) abort
  let [l:offset_to_shift, l:maybe_edited_line_numbers] = s:compute_shifts(a:path)

  let l:shift_accumulator = 0
  for l:index in range(0, len(l:offset_to_shift) - 1)
    let l:shifted_lines = l:offset_to_shift[l:index][1]
    let l:start = l:offset_to_shift[l:index][0]
    let l:end = l:index + 1 <# len(l:offset_to_shift) ? l:offset_to_shift[l:index + 1][0] : v:null
    let l:shift_accumulator += l:shifted_lines
    let l:maybe_edited_line_numbers = s:shift_locations(a:path, l:maybe_edited_line_numbers, l:start, l:end, l:shift_accumulator)
  endfor

  return len(l:offset_to_shift) + len(l:maybe_edited_line_numbers)
endf

fun! s:compute_shifts(path) abort
  let l:temp_source_path = s:temp_source_path(a:path)

  const DIFF_STATS_PATTERN = '@@ '
  const DIFF_COMMAND =
    \ 'git diff --unified=0 --ignore-cr-at-eol --ignore-space-at-eol --no-index --no-color --no-ext-diff --diff-algorithm=histogram -- '
    \ . shellescape(l:temp_source_path)
    \ . ' '
    \ . shellescape(a:path)

  let l:offset_to_shift = []
  let l:maybe_edited_line_numbers = {}
  if !filereadable(l:temp_source_path)
    return [l:offset_to_shift, l:maybe_edited_line_numbers]
  endif

  let l:diff_stdout_lines = systemlist(DIFF_COMMAND)
  let l:diff_stdout_index = 0
  while l:diff_stdout_index <# len(l:diff_stdout_lines) - 1
    let l:diff_line = l:diff_stdout_lines[l:diff_stdout_index]
    if s:starts_with(l:diff_line, DIFF_STATS_PATTERN)
      let l:raw_diff_stats = split(split(l:diff_line, DIFF_STATS_PATTERN)[0], ' ')

      let [l:removal_offset, l:removals] = s:parse_diff_stats(l:raw_diff_stats[0], '-')
      let [l:addition_offset, l:additions] = s:parse_diff_stats(l:raw_diff_stats[1], '+')
      if l:additions ==# 0 || l:removals ==# 0
        let l:shifted_lines = l:additions - l:removals
        call add(l:offset_to_shift, [l:removal_offset, l:shifted_lines])
      else
        for l:index in range(0, l:removals - 1)
          let l:maybe_edited_line_numbers[l:removal_offset + l:index] = v:true
        endfor
      endif
    endif
    let l:diff_stdout_index += 1
  endwhile

  return [l:offset_to_shift, l:maybe_edited_line_numbers]
endf

fun! s:shift_locations(path, maybe_edited_line_numbers, start, end, shift_accumulator) abort
  for l:index in range(0, len(s:editor_data.locations) - 1)
    let l:location = s:editor_data.locations[l:index]
    if l:location.path ==# a:path
      let l:current_line = l:location.line
      if l:current_line ># a:start && (a:end ==# v:null || l:current_line <# a:end)
        let s:editor_data.locations[l:index].line += a:shift_accumulator
      endif
    endif
  endfor

  for l:line in keys(a:maybe_edited_line_numbers)
    if l:line ># a:start && (a:end ==# v:null || l:line <# a:end)
      call remove(a:maybe_edited_line_numbers, l:line)
      let a:maybe_edited_line_numbers[l:line + a:shift_accumulator] = v:true
    endif
  endfor

  return a:maybe_edited_line_numbers
endf

fun! s:parse_diff_stats(text, separator) abort
  let l:offset_and_lines = split(split(a:text, a:separator)[0], ',')
  let l:offset = str2nr(l:offset_and_lines[0])
  let l:lines = len(l:offset_and_lines) ># 1 ? str2nr(l:offset_and_lines[1]) : 1
  return [l:offset, l:lines]
endf

fun! s:deduplicate_locations_by_paths_and_lines() abort
  let l:new_locations = []
  let l:added_lines = {}

  for i in s:editor_data.locations
    let l:added_line_key = string([i.path, i.line])
    let l:is_added_line = get(l:added_lines, l:added_line_key)
    if !l:is_added_line
      call add(l:new_locations, i)
      let l:added_lines[l:added_line_key] = 1
    endif
  endfor

  let s:editor_data.locations = l:new_locations
endfun

fun! s:read_all_locations_texts() abort
  for l:index in range(0, len(s:editor_data.locations) - 1)
    let l:location = s:editor_data.locations[l:index]
    let l:text = s:read_text(l:location)
    if l:text !=# v:null
      let s:locations_texts[l:index] = l:text
    endif
  endfor
endf

fun! s:is_same_as_current_location(target) abort
  let l:location = s:current_location()
  return l:location.path ==# a:target.path && l:location.line ==# a:target.line
endf

fun! s:is_current_location_edited() abort
  return has_key(s:locations_texts, s:location_index) && s:locations_texts[s:location_index] !=# s:read_text(s:current_location())
endf

fun! s:read_text(location) abort
  const MAX_LEN = 255
  let l:buf = bufnr(a:location.path)
  return l:buf ># 0 ? trim(getbufline(l:buf, a:location.line)[0][:MAX_LEN]) : v:null
endf

fun! s:maybe_delete_dead_unix_socket(server_address) abort
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
endf

fun! s:recreate_temp_sources_dir() abort
  if s:temp_sources_dir !=# v:null
    call delete(s:temp_sources_dir, 'rf')
    call mkdir(s:temp_sources_dir, 'p', 0700)
  endif
endf

fun! s:temp_source_path(path) abort
  return s:temp_sources_dir . '/' . s:escape_path(a:path)
endf

fun! s:maybe_copy_to_temp(path) abort
  call s:maybe_copy(a:path, s:temp_source_path(a:path))
endf

fun! s:maybe_copy(source, destination) abort
  const MAX_SIZE_BYTES = 1024 * 1024
  if getfsize(a:source) <=# MAX_SIZE_BYTES
    let l:data = readblob(a:source)
    call writefile(l:data, a:destination, 'b')
  endif
endf

fun! s:jump_to_location(location_index) abort
  let l:location = s:editor_data.locations[a:location_index]

  let l:current_file = s:current_file()
  if l:current_file !=# l:location.path
    execute 'silent! tab drop ' . fnameescape(l:location.path)
  end

  let l:current_position = getpos('.')
  if l:current_position[1] !=# l:location.line || current_position[2] !=# l:location.column
    call cursor((l:location.line), (l:location.column))
  endif
endf

fun! s:current_location() abort
  return s:editor_data.locations[s:location_index]
endf

fun! s:on_buffer_write() abort
  let l:current_file = s:current_file()
  if l:current_file !=# '' && filereadable(l:current_file)
    let l:has_changes = s:update_locations(l:current_file)
    if l:has_changes
      call s:maybe_copy_to_temp(l:current_file)
      let s:editor_data.corrected_locations = v:true
      call g:CargoLimitUpdate(s:editor_data)
    endif
  endif
endf

fun! s:current_file() abort
  return resolve(expand('%:p'))
endf

fun! s:escape_path(path) abort
  return substitute(a:path, '[/\\:]', '%', 'g')
endf

fun! s:starts_with(text, pattern) abort
  return stridx(a:text, a:pattern) ==# 0
endf

fun! s:contains_str(text, pattern) abort
  return stridx(a:text, a:pattern) !=# -1
endf

fun! s:log_error(...) abort
  if g:CargoLimitVerbosity >=# 1
    echohl Error
    redraw
    echon s:log_str(a:000)
    echohl None
  endif
endf

fun! s:log_warn(...) abort
  if g:CargoLimitVerbosity >=# 2
    echohl WarningMsg
    echon s:log_str(a:000) . "\n"
  endif
endf

fun! s:log_info(...) abort
  if g:CargoLimitVerbosity >=# 3
    echohl None
    echon s:log_str(a:000)
  endif
endf

fun! s:log_str(args) abort
  return '[cargo-limit] ' . join(a:args, ' ')
endf

call s:main()

" vim:shiftwidth=2 softtabstop=2 tabstop=2
