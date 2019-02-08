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
Show the version control history for the source language file.
You must specify an old-commit, and optionally, a new-commit.
###### vcs-diff
Diff two commits of the source language file.
You must specify an old-commit, and optionally, a new-commit.
###### pull
Generate translation files for all languages. 
Translation files will contain all keys that have not been translated from the source language.
If you specify a commit range (with old-commit, and optionally, new-commit), translation files will contain all keys that have either not been translated from the source language, or have been updated in the source language.
###### push
Merge all the translation files in the working directory back into version control.

##### other commands
There are other useful commands available that perform operations on individual files using the cirup engine. Check the command line help.
