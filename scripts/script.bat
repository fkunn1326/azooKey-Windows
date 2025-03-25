@REM net use Z: "\\vmware-host\Shared Folders" /persistent:yes
regsvr32.exe "D:\azookey-windows\build\azookey_windows.dll" /u /s
regsvr32.exe "D:\azookey-windows\build\x86\azookey_windows.dll" /u /s
start D:\azookey-windows\build\ui.exe
start cmd /k "set PATH=D:\azookey-windows\build\llama_cuda;%PATH% && D:\azookey-windows\build\azookey-server.exe"
regsvr32.exe "D:\azookey-windows\build\azookey_windows.dll" /s
regsvr32.exe "D:\azookey-windows\build\x86\azookey_windows.dll" /s