let s:data_chunks = []
let s:source_files = []

function! s:on_cargo_metadata(_job_id, data, event)
  if a:event == 'stdout'
    call add(s:data_chunks, join(a:data, ''))
  elseif a:event == 'stderr' && type(a:data) == v:t_list && a:data != ['']
    let l:stderr = join(a:data, "\n")
    if l:stderr !~ 'could not find `Cargo.toml`'
      echohl Error
      echon l:stderr
      echohl None
    endif
  elseif a:event == 'exit'
    let l:stdout = join(s:data_chunks, '')
    if len(l:stdout) > 0
      let l:metadata = json_decode(l:stdout)
      let l:workspace_root = get(l:metadata, 'workspace_root')
      let l:escaped_workspace_root = substitute(workspace_root, '[/\\:]', '%', 'g')
      let l:server_address = s:create_server_address(l:escaped_workspace_root)
      if !filereadable(l:server_address)
        call serverstart(l:server_address)
      endif
    endif
  endif
endfunction

function! s:create_server_address(escaped_workspace_root)
  let l:prefix = 'nvim-cargo-limit-'
  if has('win32')
    return '\\.\pipe\' . l:prefix . $USERNAME . '-' . a:escaped_workspace_root
  elseif has('unix')
    let l:server_address_dir =  '/tmp/' . l:prefix . $USER
    call mkdir(l:server_address_dir, 'p', 0700)
    let l:server_address = l:server_address_dir . '/' . a:escaped_workspace_root
    return l:server_address
  else
    throw 'unsupported OS'
  endif
endfunction

function! s:starts_with(longer, shorter)
  return a:longer[0 : len(a:shorter) - 1] ==# a:shorter
endfunction

function! s:parse_line_and_delta(text)
  let l:items = split(a:text, ',')
  let result = {'line': l:items[0][1:], 'delta': 0}
  if len(l:items) >= 2
    let result.delta = l:items[1] - 1
  endif
  return result
endfunction

function! s:correct_lines(lines_changed)
  " FIXME: ungly but works; filter does something weird
  let s:new_source_files = []
  for i in s:source_files
    let l:is_changed_file = get(a:lines_changed, i['line']) && i['path'] == l:initial_file
    if l:is_changed_file
      " TODO: naming
      for j in l:lines_deltas
        let l:new_line = i['line']
        if l:new_line >= j[0]
          let l:new_line -= j[1]
        endif
        let i.line = l:new_line
      endfor
    else
      call add(s:new_source_files, i)
    endif
  endfor
  let s:source_files = s:new_source_files
endfunction

function! s:on_buffer_changed()
  let l:initial_file = resolve(expand('%:p'))
  if l:initial_file != ''
    if filereadable(l:initial_file)


      " TODO: extract parsing
      let l:diff_stdout_lines = split(execute('w !git diff --unified=0 --ignore-all-space --no-index --no-color --no-ext-diff % -'), "\n")
      let l:diff_change_pattern = '@@ '
      let l:lines_changed = {}
      let l:lines_deltas = []
      let l:lines_moved = {}
      let l:diff_stdout_line_number = 0
      while l:diff_stdout_line_number < len(l:diff_stdout_lines) - 1
        let l:diff_line = l:diff_stdout_lines[l:diff_stdout_line_number]
        if s:starts_with(l:diff_line, l:diff_change_pattern)
          let l:changed_line_numbers_with_offsets = trim(split(l:diff_line, l:diff_change_pattern)[0])
          " TODO: naming
          let l:wat = split(l:changed_line_numbers_with_offsets, ' ')
          let l:removed = s:parse_line_and_delta(l:wat[0])
          let l:removed_source_file_line = l:removed.line
          let l:added = s:parse_line_and_delta(l:wat[1])
          let l:delta = l:added.delta - l:removed.delta
          call add(l:lines_deltas, [l:removed.line, l:delta])

          let l:next_diff_line = l:diff_stdout_lines[l:diff_stdout_line_number + 1]
          let l:removed_text = l:next_diff_line[1:]
          if !empty(l:removed_text)
            let l:lines_changed[l:removed_source_file_line] = 1
          endif
          let l:diff_stdout_line_number += 1
        endif
        let l:diff_stdout_line_number += 1
      endwhile

      call s:correct_lines(l:lines_changed)

    endif
  endif
endfunction

" TODO: naming
function! s:open_next_source_file_in_new_or_existing_tab(allow_not_normal_mode)
  " TODO: naming: current_file?
  let l:initial_file = resolve(expand('%:p'))
  if l:initial_file != '' && !filereadable(l:initial_file)
    return
  endif

  " TODO: naming?
  if !a:allow_not_normal_mode
    "for source_file in reverse(s:source_files)
    for source_file in s:source_files
      let l:path = fnameescape(source_file.path)
      if mode() == 'n' && &l:modified == 0
        execute 'tab drop ' . l:path
        call cursor((source_file.line), (source_file.column))
      else
        break
      endif
    endfor
    " FIXME: why second reverse?
    let s:source_files = reverse(s:source_files)[1:]
    return
  endif

  if !empty(s:source_files)
    let l:source_file = s:source_files[0]
    let l:path = fnameescape(l:source_file.path)
    let l:allowed_mode = a:allow_not_normal_mode || mode() == 'n'
    if l:allowed_mode && &l:modified == 0
      execute 'tab drop ' . l:path
      call cursor((l:source_file.line), (l:source_file.column))
      let s:source_files = s:source_files[1:]
    endif
  endif
endfunction

function! s:open_source_files_sequentially(editor_data)
  let s:source_files = reverse(a:editor_data.files)
  call s:open_next_source_file_in_new_or_existing_tab(0)
endfunction

function! s:call_after_event_finished(function)
  call timer_start(0, { tid -> a:function() })
endfunction

if !exists('*CargoLimitOpen')
  function! g:CargoLimitOpen(editor_data)
    call s:open_source_files_sequentially(a:editor_data)
  endfunction

  " TODO: augroup?
  autocmd TextChanged,InsertLeave,FilterReadPost *.rs call s:on_buffer_changed()

  autocmd BufWritePre *.rs call s:call_after_event_finished(
    \ {-> execute('call s:open_next_source_file_in_new_or_existing_tab(1)') })
endif

if has('nvim')
  call jobstart(['cargo', 'metadata', '--quiet', '--format-version=1'], {
  \ 'on_stdout': function('s:on_cargo_metadata'),
  \ 'on_stderr': function('s:on_cargo_metadata'),
  \ 'on_exit': function('s:on_cargo_metadata'),
  \ })
else
  throw 'unsupported text editor'
endif

" vim:shiftwidth=2 softtabstop=2 tabstop=2
