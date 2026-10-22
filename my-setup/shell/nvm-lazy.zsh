export NVM_DIR="$HOME/.nvm"
# Loading nvm.sh costs ~300ms per shell, so the default Node goes straight on PATH and nvm itself
# loads the first time it is called. Falls back to loading it now if the default isn't an exact version.
_nvm_default_bin="$NVM_DIR/versions/node/$(<"$NVM_DIR/alias/default")/bin"
if [ -d "$_nvm_default_bin" ]; then
  export PATH="$_nvm_default_bin:$PATH"
  nvm() {
    unfunction nvm
    \. "$NVM_DIR/nvm.sh"
    [ -s "$NVM_DIR/bash_completion" ] && \. "$NVM_DIR/bash_completion"
    nvm "$@"
  }
else
  [ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"
  [ -s "$NVM_DIR/bash_completion" ] && \. "$NVM_DIR/bash_completion"
fi
unset _nvm_default_bin
