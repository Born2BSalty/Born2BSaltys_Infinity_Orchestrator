cd /d "%~dp0"
git add -A
git commit -m "Update"
git pull --rebase origin main
git push origin main