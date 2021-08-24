"TODO: use s: in plugin
"TODO: detect OS, set named pipe on windows
"TODO: socket permissions? put it to dir with specific permissions /tmp/nvim-username/%home...
"TODO: escape paths with spaces and weird characters?
"TODO: escape windows username?
"TODO: detect whether cargo installed?
function! s:on_cargo_metadata_stdout(_job_id, data, _event)
  let l:stdout = trim(join(a:data, ''))
  if len(l:stdout) > 0
    let l:metadata = json_decode(l:stdout)
    let l:workspace_root = get(l:metadata, 'workspace_root')
    let l:escaped_workspace_root = substitute(workspace_root, '/', '%', 'g')

    let l:socket_dir = '/tmp/nvim-cargo-limit-' . $USER
    call mkdir(l:socket_dir, 'p', 0700)

    let l:socket_path = l:socket_dir . '/' . l:escaped_workspace_root
    if !filereadable(l:socket_path)
      call serverstart(l:socket_path)
    endif
  endif
endfunction

if has('nvim')
  call jobstart(['cargo', 'metadata', '--quiet', '--format-version=1'], {
  \ 'on_stdout': function('s:on_cargo_metadata_stdout')
  \ })
else
  throw 'unsupported text editor'
endif

" vim:shiftwidth=2 softtabstop=2 tabstop=2
