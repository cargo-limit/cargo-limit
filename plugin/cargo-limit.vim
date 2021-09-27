"TODO: escape windows username? probably no
"TODO: detect whether cargo installed?
"TODO: stderr => print error?

let s:data_chunks = []

function! s:on_cargo_metadata_stdout(_job_id, data, event)
  if a:event == 'stdout'
    call add(s:data_chunks, join(a:data, ''))
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

" TODO: pass a list of files?
function! s:open_in_new_or_existing_tab(path, line, column)
  " TODO: don't handle this stuff if
  " + current mode is not normal
  " - or no file from project is currently open and active (which means it's netrw/fzf search/etc. is going on)
  " - or current file is unsaved (which means it's probably been edited using normal mode)
  " - or normal mode input buffer is not empty (for instance we're in the middle on `2G` command)
  " - or command line buffer is not empty (or just : is in the input)
  " - or search mode is active (/...)
  if mode() == 'n'
    "TODO: escape path here?
    "call feedkeys('<esc>:tab drop ' . path . '<cr>' . line . 'G' . column . '|')
  endif
endfunction

if has('nvim')
  call jobstart(['cargo', 'metadata', '--quiet', '--format-version=1'], {
  \ 'on_stdout': function('s:on_cargo_metadata_stdout'),
  \ 'on_exit': function('s:on_cargo_metadata_stdout'),
  \ })
else
  throw 'unsupported text editor'
endif

" vim:shiftwidth=2 softtabstop=2 tabstop=2
