@REM net use Z: "\\vmware-host\Shared Folders" /persistent:yes
start D:\azookey-windows\target\debug\azookey-server.exe
start D:\azookey-windows\target\debug\ui.exe
"C:\Program Files\Azookey\unins000.exe"
target\release\azookey-installer.exe