# Read from the Cellar instead of `brew ls --versions`, which cost ~330ms per shell.
installedPhpVersions=()
for _php_dir in /opt/homebrew/Cellar/php{,@*}(N/); do
  for _php_ver in $_php_dir/*(N/:t); do installedPhpVersions+=(${(M)_php_ver#[0-9].[0-9]}); done
done
installedPhpVersions=(${(ou)installedPhpVersions})
unset _php_dir _php_ver
