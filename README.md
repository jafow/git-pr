# git-pr
plugin to open a pull-request from the command line

# usage

open a pull request from current branch to master branch on origin
```bash
git pr origin
```

open a pull request to remote "upstream" on `feat/new-feature` branch:
```bash
git pr upstream feat/new-feature
```

open a pull request using message passed on command line
```bash
git pr origin master -m "My pull request title"
```

# install
get the binary from github release

TODO: @jafow


# LICENSE
APACHE2.0 & MIT
