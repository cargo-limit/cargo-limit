let s:diff_new_changes_command =
  \ 'w !git diff --unified=0 --ignore-all-space --no-index --no-color --no-ext-diff % -'
let s:diff_change_pattern = '@@ '

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

function! s:on_buffer_changed()
  let l:current_file = s:current_file()
  if l:current_file != '' && filereadable(l:current_file)
    let changed_line_numbers = s:compute_changed_line_numbers()
    call s:ignore_changed_lines_of_current_file(changed_line_numbers, l:current_file)
  endif
endfunction

function! s:open_all_files_in_new_or_existing_tabs(files)
  let s:source_files = reverse(a:files)
  for source_file in s:source_files
    let l:path = fnameescape(source_file.path)
    if mode() == 'n' && &l:modified == 0
      execute 'tab drop ' . l:path
      call cursor((source_file.line), (source_file.column))
    else
      break
    endif
  endfor
  let s:source_files = reverse(s:source_files)[1:]
endfunction

function! s:open_next_file_in_new_or_existing_tab()
  let l:current_file = s:current_file()
  if l:current_file == '' || filereadable(l:current_file) && !empty(s:source_files)
    let l:source_file = s:source_files[0]
    let l:path = fnameescape(l:source_file.path)
    if &l:modified == 0
      execute 'tab drop ' . l:path
      call cursor((l:source_file.line), (l:source_file.column))
      let s:source_files = s:source_files[1:]
    endif
  endif
endfunction

function! s:ignore_changed_lines_of_current_file(changed_line_numbers, current_file)
  let s:new_source_files = []
  for i in s:source_files
    let l:is_changed_line = get(a:changed_line_numbers, i.line) && i.path == a:current_file
    if !l:is_changed_line
      call add(s:new_source_files, i)
    endif
  endfor
  let s:source_files = s:new_source_files
endfunction

function! s:compute_changed_line_numbers()
  let l:changed_line_numbers = {}
  let l:diff_stdout_lines = split(execute(s:diff_new_changes_command), "\n")

  function! s:parse_line_number(text)
    return split(a:text, ',')[0][1:]
  endfunction

  let l:diff_stdout_line_number = 0
  while l:diff_stdout_line_number < len(l:diff_stdout_lines) - 1
    let l:diff_line = l:diff_stdout_lines[l:diff_stdout_line_number]
    if s:starts_with(l:diff_line, s:diff_change_pattern)
      let l:changed_line_numbers_with_offsets = trim(split(l:diff_line, s:diff_change_pattern)[0])
      let l:removed_line = s:parse_line_number(split(l:changed_line_numbers_with_offsets, ' ')[0])
      let l:next_diff_line = l:diff_stdout_lines[l:diff_stdout_line_number + 1]
      let l:removed_text = l:next_diff_line[1:]
      let l:removed_new_line = empty(l:removed_text)
      if !l:removed_new_line
        let l:changed_line_numbers[l:removed_line] = 1
      endif
      let l:diff_stdout_line_number += 1
    endif
    let l:diff_stdout_line_number += 1
  endwhile

  return l:changed_line_numbers
endfunction

function! s:current_file()
  return resolve(expand('%:p'))
endfunction

function! s:starts_with(longer, shorter)
  return a:longer[0 : len(a:shorter) - 1] ==# a:shorter
endfunction

function! s:call_after_event_finished(function)
  call timer_start(0, { tid -> a:function() })
endfunction

if !exists('*CargoLimitOpen')
  function! g:CargoLimitOpen(editor_data)
    call s:open_all_files_in_new_or_existing_tabs(a:editor_data.files)
  endfunction

  augroup CargoLimitAutocommands
    autocmd!
    autocmd TextChanged,InsertLeave,FilterReadPost *.rs call s:on_buffer_changed()
    autocmd BufWritePre *.rs call s:call_after_event_finished(
      \ {-> execute('call s:open_next_file_in_new_or_existing_tab()') })
  augroup END
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
