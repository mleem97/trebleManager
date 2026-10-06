@echo off
setlocal EnableExtensions
title trebleManager - Push all (repo + wiki)

REM Push main repo + wiki subrepo in sequence (step 4 of the release flow).
set "BRANCH=%~1"
if "%BRANCH%"=="" set "BRANCH=main"
git push origin %BRANCH% || exit /b 1
git -C wiki push origin master || exit /b 1
echo Pushed: main repo (%BRANCH%) + wiki (master).
