# Log Archiver
A program to archive files based on how old are they, which is determined via data from CLI arguments (see `--help`). Also, you supply a date when files get too old and need to be deleted.  
Is recursively traverses each directory inside specifed directory and packs it's contents to archives via this format:
```
dirName/dirName_dd-mm-yy.zip
```
> Original files will be removed

# Building
## Installation
- **On Windows**:
You need to have use [rustup](https://www.rust-lang.org/tools/install) to install rust on your system, as well as the [C++ Build tools](https://visualstudio.microsoft.com/visual-cpp-build-tools)

- **On Linux**:
For linux, use rustup and gcc, which is probably installed on your system.

## Using cargo
After you've used rustup to configure rust, you can clone this repo and run this command to build the program:
```bash
# Binary will be located in target/release/archiver
cargo build --release
```



# Future improvements (may or may not be done):
- Select extenstion to specifically target
- Select archiving format