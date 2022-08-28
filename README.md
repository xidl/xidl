#### XIDL - Handy download links for your automated scripts

## 7za.exe
[https://xidl.github.io/7za.exe](https://xidl.github.io/7za.exe)

## wget
[https://xidl.github.io/wget.exe](https://xidl.github.io/wget.exe) | [.7z](https://xidl.github.io/wget.7z)

## Usage example
`myroutinarysetup.bat`
```cmd
@echo off
if not exist "7z.exe" curl -L -sS "https://xidl.github.io/7za.exe" -o 7z.exe
if not exist "wget.exe" curl -L -sS "https://xidl.github.io/wget.7z" -o wget.7z
7z.exe x -y "wget.7z">NUL & if exist wget.exe del wget.7z

echo Downloading dependencies...
wget -nc https://example.com/dep1.zip -q --show-progress
wget -nc https://example.com/dep2.tar -q --show-progress
wget -nc https://example.com/dep3.7z -q --show-progress

echo Uncompressing files...
7z.exe x -y "dep1.zip" -o"multimediatools">NUL
7z.exe x -y "dep2.tar" -o"webtools">NUL
7z.exe e -y "dep3.7z" "bin\executable.exe" -o"tools\bin">NUL
echo Done!
pause
```
---
##### Looking for [64-bit binaries](https://xidl.github.io/64/)?
