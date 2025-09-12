fun! s:main() abort
  const MIN_NVIM_VERSION = '0.7.0'

  if has('nvim')
    if !has('nvim-' . MIN_NVIM_VERSION)
      throw 'unsupported nvim version, expected >=' . MIN_NVIM_VERSION
    end

    if !exists('g:CargoLimitVerbosity')
      let g:CargoLimitVerbosity = 3 " info level
    end
    let s:editor_data = {'locations': []}
    let s:locations_texts = {}
    let s:location_index = v:null
    let s:workspace_root = v:null
    let s:temp_dir = v:null
    let s:deprecated_cargo_limit_open = v:null
    let s:lazyredraw = &lazyredraw
    let s:allow_redraw = v:true
    call jobstart(['cargo', 'metadata', '--quiet', '--format-version=1'], {
      \ 'on_stdout': function('s:on_cargo_metadata'),
      \ 'on_stderr': function('s:on_cargo_metadata'),
      \ 'stdout_buffered': v:true,
      \ 'stderr_buffered': v:true,
      \ })
  else
    throw 'unsupported text editor'
  end
endf

fun! s:on_cargo_metadata(_job_id, data, event) abort
  if a:event ==# 'stdout'
    let l:stdout = trim(join(a:data, ''))
    if !empty(l:stdout)
      let l:metadata = json_decode(l:stdout)
      let s:workspace_root = l:metadata.workspace_root
      let l:escaped_workspace_root = s:escape_path(s:workspace_root)
      call s:start_server(l:escaped_workspace_root)
    end
  elseif a:event ==# 'stderr'
    let l:stderr = trim(join(a:data, "\n"))
    if !empty(l:stderr) && !s:contains_str(l:stderr, 'could not find `Cargo.toml`')
      call s:log_error('cargo metadata', l:stderr)
    end
  end
endf

fun! s:start_server(escaped_workspace_root) abort
  const TEMP_DIR_PREFIX = 'nvim-cargo-limit-'

  if has('unix')
    let s:temp_dir = '/tmp/' . TEMP_DIR_PREFIX . $USER
    let l:server_address = s:temp_dir . '/' . a:escaped_workspace_root
    call s:maybe_delete_dead_unix_socket(l:server_address)
  elseif has('win32')
    let l:server_address_postfix = TEMP_DIR_PREFIX . $USERNAME . '-' . a:escaped_workspace_root
    let l:server_address = '\\.\pipe\' . l:server_address_postfix
  else
    throw 'unsupported OS'
  end

  if !filereadable(l:server_address)
    call s:maybe_create_temp_dir()
    call s:maybe_setup_handlers()
    call serverstart(l:server_address)
    call s:log_info('ready')
  end
endf

fun! s:maybe_setup_handlers() abort
  augroup CargoLimitAutocommands
    autocmd!
    autocmd VimLeavePre * call s:maybe_create_temp_dir()
    autocmd BufWritePost *.rs call s:on_buffer_write(expand('<afile>:p'))
  augroup END

  if exists('*CargoLimitOpen')
    let s:deprecated_cargo_limit_open = funcref('g:CargoLimitOpen')
    call s:log_warn(
      \ 'g:CargoLimitOpen is deprecated, please migrate to g:CargoLimitUpdate:',
      \ 'https://github.com/cargo-limit/cargo-limit#text-editoride-integrations'
      \ )
  end

  fun! g:CargoLimitOpen(editor_data) abort
    let s:editor_data = a:editor_data
    let s:locations_texts = {}

    if s:deprecated_cargo_limit_open !=# v:null
      call s:downgrade_editor_data_format()
      call s:deprecated_cargo_limit_open(s:editor_data)
    end

    call s:upgrade_editor_data_format()

    if s:deprecated_cargo_limit_open !=# v:null
      call s:finalize_locations()
      return
    end

    if !exists('*CargoLimitUpdate')
      fun! g:CargoLimitUpdate(editor_data) abort
        let l:current_file = s:current_file()
        if empty(s:editor_data.locations) || a:editor_data.corrected_locations || (l:current_file !=# '' && !filereadable(l:current_file))
          return
        end

        call s:deduplicate_locations_by_paths_and_lines()
        call s:open_all_locations_in_reverse()
      endf
    end

    call g:CargoLimitUpdate(s:editor_data)
    call s:finalize_locations()
  endf

  fun! g:CargoLimitOpenNextLocation() abort
    call s:switch_location(function('s:increment_location_index'))
  endf

  fun! g:CargoLimitOpenPrevLocation() abort
    call s:switch_location(function('s:decrement_location_index'))
  endf
endf

fun! s:downgrade_editor_data_format() abort
  if exists('s:editor_data.locations')
    let s:editor_data.files = s:editor_data.locations
    call remove(s:editor_data, 'locations')
    call remove(s:editor_data, 'corrected_locations')
  end
  if !exists('s:editor_data.files')
    let s:editor_data.files = []
  end
endf

fun! s:upgrade_editor_data_format() abort
  if exists('s:editor_data.files')
    let s:editor_data.locations = s:editor_data.files
    call remove(s:editor_data, 'files')
  end
  if !exists('s:editor_data.locations')
    let s:editor_data.locations = []
  end
  let s:editor_data.corrected_locations = exists('s:editor_data.corrected_locations') && s:editor_data.corrected_locations ? v:true : v:false
endf

fun! s:open_all_locations_in_reverse() abort
  let l:path_to_location_index = {}
  for l:index in range(len(s:editor_data.locations) - 1, 0, -1)
    let l:path_to_location_index[s:editor_data.locations[l:index].path] = l:index
  endfor

  call s:disable_redraw()
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
    end
  endfor
  call s:enable_redraw()
endf

fun! s:increment_location_index() abort
  let l:initial_location = s:current_location()
  for s:location_index in range(s:location_index + 1, len(s:editor_data.locations) - 1)
    if !s:is_same_as_current_location(l:initial_location) && !s:is_current_location_edited()
      break
    end
  endfor
endf

fun! s:decrement_location_index() abort
  let l:initial_location = s:current_location()
  for s:location_index in range(s:location_index - 1, 0, -1)
    if !s:is_same_as_current_location(l:initial_location) && !s:is_current_location_edited()
      break
    end
  endfor
endf

fun! s:on_buffer_write(path) abort
  if empty(s:editor_data.locations) || a:path ==# '' || !filereadable(a:path)
    return
  end

  let s:editor_data.corrected_locations = s:update_locations(a:path)
  if s:editor_data.corrected_locations && exists('*CargoLimitUpdate')
    call g:CargoLimitUpdate(s:editor_data)
  end
endf

fun! s:update_locations(path) abort
  const MAX_LINES = 16 * 1024

  let l:old_locations = deepcopy(s:editor_data.locations, 1)
  let l:found_lines = {}
  let l:shift = 0
  let l:bufinfo = s:bufinfo_if_loaded(bufnr(a:path))
  let l:max_buf_line = empty(l:bufinfo) ? len(readfile(a:path)) : min([l:bufinfo.linecount, MAX_LINES])

  for l:index in range(0, len(s:editor_data.locations) - 1)
    let l:location = s:editor_data.locations[l:index]
    if l:location.path !=# a:path || !has_key(s:locations_texts, l:index)
      continue
    end
    let s:editor_data.locations[l:index].line += l:shift
    let l:location = s:editor_data.locations[l:index]

    let l:prev_line = min([l:location.line - 1, MAX_LINES])
    let l:next_line = min([l:location.line + 1, l:max_buf_line])
    let l:prev_lines = range(max([1, l:prev_line]), 1, -1)
    let l:next_lines = range(l:next_line, l:max_buf_line)

    for l:line in s:zip_flatten([l:location.line] + l:prev_lines, l:next_lines)
      if has_key(l:found_lines, l:line)
        continue
      end
      let l:text = s:read_text_by_line(a:path, l:line)
      if l:text !=# v:null && s:locations_texts[l:index] ==# l:text
        let l:shift += s:editor_data.locations[l:index].line - l:line
        let s:editor_data.locations[l:index].line = l:line
        let l:found_lines[l:line] = v:true
        break
      end
    endfor
  endfor

  eval s:editor_data.locations->sort({ a, b -> a.line ==# b.line ? a.column - b.column : a.line - b.line })
  return l:old_locations ==# s:editor_data.locations ? v:false : v:true
endf

fun! s:deduplicate_locations_by_paths_and_lines() abort
  let l:new_locations = []
  let l:added_lines = {}

  for l:i in reverse(s:editor_data.locations)
    let l:added_line_key = string([l:i.path, l:i.line])
    let l:is_added_line = get(l:added_lines, l:added_line_key)
    if !l:is_added_line
      call add(l:new_locations, l:i)
      let l:added_lines[l:added_line_key] = v:true
    end
  endfor

  let s:editor_data.locations = reverse(l:new_locations)
endfun

fun! s:finalize_locations() abort
  for l:index in range(0, len(s:editor_data.locations) - 1)
    let l:location = s:editor_data.locations[l:index]
    let l:text = s:read_text(l:location)
    if l:text !=# v:null
      let s:locations_texts[l:index] = l:text
    end
  endfor
  let s:location_index = 0
endf

fun! s:switch_location(change_location_index) abort
  echomsg ''
  let l:current_file = s:current_file()
  if &l:modified !=# 0 || empty(s:editor_data.locations) || (l:current_file !=# '' && !filereadable(l:current_file))
    return
  end

  let l:initial_location_index = s:location_index
  call a:change_location_index()
  if s:is_current_location_edited()
    let s:location_index = l:initial_location_index
  else
    call s:disable_redraw()
    call s:jump_to_location(s:location_index)
    call s:enable_redraw()
  end
endf

fun! s:jump_to_location(location_index) abort
  let l:location = s:editor_data.locations[a:location_index]

  let l:current_file = s:current_file()
  if l:current_file !=# l:location.path
    execute 'silent! tab drop ' . fnameescape(l:location.path)
  end

  let l:current_position = getpos('.')
  if l:current_position[1] !=# l:location.line || l:current_position[2] !=# l:location.column
    call cursor((l:location.line), (l:location.column))
  end
endf

fun! s:is_same_as_current_location(target) abort
  let l:location = s:current_location()
  return l:location.path ==# a:target.path && l:location.line ==# a:target.line
endf

fun! s:is_current_location_edited() abort
  if !has_key(s:locations_texts, s:location_index)
    return v:false
  end
  let l:text = s:read_text(s:current_location())
  return l:text !=# v:null && s:locations_texts[s:location_index] !=# l:text
endf

fun! s:read_text_by_line(path, line) abort
  const MAX_LENGTH = 255

  let l:buf = bufnr(a:path)
  let l:bufinfo = s:bufinfo_if_loaded(l:buf)
  let l:text = empty(l:bufinfo) ? readfile(a:path, '', a:line) : getbufline(l:buf, a:line)
  return empty(l:text) ? v:null : l:text[-1][:MAX_LENGTH]
endf

fun! s:read_text(location) abort
  return s:read_text_by_line(a:location.path, a:location.line)
endf

fun! s:bufinfo_if_loaded(buf) abort
  let l:bufinfo = getbufinfo(a:buf)
  return a:buf >=# 0 && !empty(l:bufinfo) && l:bufinfo[0].loaded ? l:bufinfo[0] : {}
endf

fun! s:current_location() abort
  return s:editor_data.locations[s:location_index]
endf

fun! s:current_file() abort
  return resolve(expand('%:p'))
endf

fun! s:escape_path(path) abort
  return substitute(a:path, '[/\\:]', '%', 'g')
endf

fun! s:contains_str(text, pattern) abort
  return stridx(a:text, a:pattern) !=# -1
endf

fun! s:maybe_delete_dead_unix_socket(server_address) abort
  const LSOF_EXECUTABLE = 'lsof'
  const LSOF_COMMAND = LSOF_EXECUTABLE . ' -U'

  if !filereadable(a:server_address)
    return
  end

  call system('which ' . LSOF_EXECUTABLE)
  let l:lsof_is_installed = v:shell_error ==# 0
  if !l:lsof_is_installed
    return
  end

  let l:lsof_stdout = system(LSOF_COMMAND)
  let l:lsof_succeed = v:shell_error ==# 0
  if l:lsof_succeed
    let l:socket_is_dead = !s:contains_str(l:lsof_stdout, a:server_address)
    if l:socket_is_dead
      let l:ignore = luaeval('os.remove(_A)', a:server_address)
      call s:log_info('removed dead socket', a:server_address)
    end
  else
    call s:log_error('failed to execute', LSOF_COMMAND)
  end
endf

fun! s:maybe_create_temp_dir() abort
  if s:temp_dir !=# v:null
    call mkdir(s:temp_dir, 'p', 0700)
    call setfperm(s:temp_dir, 'rwx------')
  end
endf

fun! s:disable_redraw() abort
  if !s:allow_redraw
    return
  end
  let s:allow_redraw = v:false
  let s:lazyredraw = &lazyredraw
  set lazyredraw
endf

fun! s:enable_redraw() abort
  if s:allow_redraw
    return
  end
  let s:allow_redraw = v:true
  let &lazyredraw = s:lazyredraw
  redraw!
endf

fun! s:zip_flatten(xs, ys) abort
  let l:result = []
  let l:i = 0
  while l:i < max([len(a:xs), len(a:ys)])
    if l:i < len(a:xs)
      call add(l:result, a:xs[l:i])
    endif
    if l:i < len(a:ys)
      call add(l:result, a:ys[l:i])
    endif
    let l:i += 1
  endw
  return l:result
endf

fun! s:log_error(...) abort
  if g:CargoLimitVerbosity >=# 1
    echohl Error
    redraw
    echon s:log_str(a:000)
    echohl None
  end
endf

fun! s:log_warn(...) abort
  if g:CargoLimitVerbosity >=# 2
    echohl WarningMsg
    echon s:log_str(a:000) . "\n"
  end
endf

fun! s:log_info(...) abort
  if g:CargoLimitVerbosity >=# 3
    echohl None
    echon s:log_str(a:000)
  end
endf

fun! s:log_str(args) abort
  return '[cargo-limit] ' . join(a:args, ' ')
endf

call s:main()

" vim:shiftwidth=2 softtabstop=2 tabstop=2
