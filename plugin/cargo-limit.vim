"TODO: detect OS, set named pipe on windows
"TODO: escape paths with spaces and weird characters?
"TODO: escape windows username?
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
      let l:escaped_workspace_root = substitute(workspace_root, '/', '%', 'g')

      let l:server_address_dir = '/tmp/nvim-cargo-limit-' . $USER
      call mkdir(l:server_address_dir, 'p', 0700)

      let l:server_address_path = l:server_address_dir . '/' . l:escaped_workspace_root
      if !filereadable(l:server_address_path)
        call serverstart(l:server_address_path)
      endif
    endif
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
