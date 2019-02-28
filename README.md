# cirup-rs

A translation continuous integration tool

##### configuration
A configuration file is required for most operations:
 ```
[vcs]
# The version control system to use
plugin = "git"
# The local path for the repository
local_path = "/opt/wayk/i18n/WaykNow" 
# The remote path to the repository
remote_path = "git@bitbucket.org:devolutions/wayknow.git"

[sync]
# The source language
source_language = "en"
# The target language(s)
target_languages = [ "fr", "de" ]
# A regex to match your language files
match_language_file = "\\.json$"
# A regex to match the language from the language filename
match_language_name = "(.+?)(\\.[^.]*$|$)"
# The relative path to the language files in the repository
source_dir = "resources/i18n"
# The location to export and import translations from
working_dir = "/opt/wayk/i18n/WaykNow-Translations"
```
##### commands
###### vcs-log
Show the version control history for the source language file. You must specify an old commit, and optionally, a new commit.

Commits are listed, newest first, and formatted as:
`%commit - %date - %author - %message`

You can limit the number of commits returned with `--limit`

e.g. `cirup vcs-log --old-commit ac8d579fd --limit 20`

###### vcs-diff
Diff two commits of the source language file. You must specify an old commit, and optionally, a new commit.
###### pull
Generate translation files for all target languages. You can specify a commit range.
Translation files will contain all keys that have not been translated from the source language. You can also include strings that have changed in the commit range using `--show-changes`.

e.g. `cirup pull --old-commit ac8d579fd --show-changes`
###### push
Merge the translation files in the working directory back into version control.
You can specify a commit range to merge a specific set of changes.
##### other commands
There are other useful commands available that perform operations on individual files using the cirup engine. Check the command line help.
