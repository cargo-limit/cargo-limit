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

function! s:open_next_source_file_in_new_or_existing_tab()
  let l:initial_file = resolve(expand('%:p'))
  if l:initial_file != '' && !filereadable(l:initial_file)
    return
  endif

  " TODO
  " :w !git diff --no-index % -
  " 1. remove edited lines from s:source_files
  " 2. correct line numbers

  if !empty(s:source_files)
    let l:source_file = s:source_files[0]
    let l:path = fnameescape(l:source_file.path)
    if mode() == 'n' && &l:modified == 0
      execute 'tab drop ' . l:path
      call cursor((l:source_file.line), (l:source_file.column))
      let s:source_files = s:source_files[1:]
    endif
  endif
endfunction

function! s:open_source_files_sequentially(editor_data)
  let s:source_files = a:editor_data.files
  call s:open_next_source_file_in_new_or_existing_tab()
endfunction

function! s:call_after_event_finished(function)
  call timer_start(0, { tid -> a:function() })
endfunction

if !exists('*CargoLimitOpen')
  function! g:CargoLimitOpen(editor_data)
    call s:open_source_files_sequentially(a:editor_data)
  endfunction

  autocmd BufWritePre *.rs call s:call_after_event_finished(
    \ {-> execute('call s:open_next_source_file_in_new_or_existing_tab()') })
  " TODO: function('s:open_next_source_file_in_new_or_existing_tab')
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
