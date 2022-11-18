# Log Archiver
A program to archive files based on how old are they, which is determined via data from CLI arguments (see `--help`). Also, you supply a date when files get too old and need to be deleted.  
Is recursively traverses each directory inside specifed directory and packs it's contents to archives via this format:
```
dirName/dirName_dd-mm-yy.zip
```
> Original files will be removed

# Future improvements (may or may not be done):
- Select extenstion to specifically target
- Select archiving format