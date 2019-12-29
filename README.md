# git-pr
plugin to open a pull-request from the command line

## todo

- [x] when given branch-only option get the current working branch and origin
  url
- [x] if -m is passed launch $EDITOR
- [ ] check the token
    - [v1.0.0]if not available check for an .env file
    - error if not
- [x] construct the url


- write the PR mesage template before launch editor, construct the
  `PullRequestMsg` type before preparing the POST

- handle errors in fetch



# LICENSE
APACHE2.0 & MIT
