// This file would not be possible without Nekear
use chrono::{DateTime, Datelike, Duration, Local, Timelike};
use clap::Parser;
use zip::write::FileOptions;
use std::{
    fs::{self, DirEntry},
    path::PathBuf,
    process::exit,
    time::Instant, io::Write,
};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(required = true)]
    directory: PathBuf,

    /// Archive files older than <archive> days old
    #[clap(short, long)]
    archive: usize,

    /// Delete files older than <delete> days old
    #[clap(short, long)]
    delete: usize,
}

fn main() {
    let now = Instant::now();
    let args = Args::parse();

    if !args.directory.exists() {
        println!("Directory {:#?} does not exist.", args.directory);
        exit(1);
    }

    if args.delete < args.archive {
        println!("Amount of days for archivation should be less than amount of days for deletion!");
        exit(1);
    }

    let local_time = chrono::offset::Local::now();
    // Add -1 becuase of partition point algorithm finds index where it's "given date and less",
    // and we need strictly less than gived date
    let archive_from = local_time - Duration::days(args.archive as i64 - 1);
    let archive_from = normalize_date(&archive_from);

    let delete_from = local_time - Duration::days(args.delete as i64);
    let delete_from = normalize_date(&delete_from);

    let processed = process_dir(&args.directory, &archive_from, &delete_from);

    println!("Done\n{:.2}s\n{} files", now.elapsed().as_secs_f32(), processed)
}

fn list_dir_files(path: &PathBuf) -> Vec<DirEntry> {
    return fs::read_dir(path)
        .unwrap()
        .map(|v| v.unwrap())
        .filter(|v| v.path().is_file())
        .collect();
}

fn list_subdirs(path: &PathBuf) -> Vec<DirEntry> {
    return fs::read_dir(path)
        .unwrap()
        .map(|v| v.unwrap())
        .filter(|v| v.path().is_dir())
        .collect();
}

fn normalize_date(date: &DateTime<Local>) -> DateTime<Local> {
    return date
        .with_hour(0)
        .unwrap()
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap();
}

fn is_same_day(a: &DateTime<Local>, b: &DateTime<Local>) -> bool {
    return a.year() == b.year() && a.month() == b.month() && a.day() == b.day();
}

fn pack_to_archive(files: &Vec<&DirEntry>, dir: &PathBuf, date: &DateTime<Local>) {
    let human_readable = date.format("%d-%m-%Y");
    let dest = dir.join(format!(
        "{}_{}.zip",
        dir.file_name().unwrap().to_str().unwrap(),
        human_readable
    ));

    let file = fs::File::create(dest).unwrap();
    // let mut tar = TarBuilder::new(file);
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::DEFLATE);

    // Move all files to newly created directory
    
    for v in files {
        zip.start_file(v.file_name().to_string_lossy(), options).unwrap();
        zip.write_all(&fs::read(v.path()).unwrap()).unwrap();
        fs::remove_file(v.path()).unwrap();
    }

    zip.finish().unwrap();
}

fn archive_files(files: &[DirEntry], parent_dir: &PathBuf) -> usize {
    if files.len() < 1 {
        return 0;
    }

    let mut current_date: DateTime<Local> = files[0].metadata().unwrap().modified().unwrap().into();
    let mut files_to_archive: Vec<&DirEntry> = vec![];
    let mut amount = 0;

    for f in files.iter() {
        // Skip archives
        if f.file_name().to_string_lossy().ends_with(".zip") {
            continue;
        }

        let date: DateTime<Local> = f.metadata().unwrap().modified().unwrap().into();

        if is_same_day(&date, &current_date) {
            files_to_archive.push(f);
            continue;
        }

        amount += files_to_archive.len();

        pack_to_archive(&files_to_archive, &parent_dir, &current_date);
        // Reset for the next date
        files_to_archive.clear();
        current_date = date.clone();

        files_to_archive.push(f);
    }

    // Archive last date
    amount += files_to_archive.len();
    pack_to_archive(&files_to_archive, &parent_dir, &current_date);


    return amount;
}

fn process_dir(dir: &PathBuf, archive_from: &DateTime<Local>, delete_from: &DateTime<Local>) -> usize {
    // Sort files by modified time
    let mut files = list_dir_files(dir);
    files.sort_by(|a, b| {
        let a_upd = a.metadata().unwrap().modified().unwrap();
        let b_upd = b.metadata().unwrap().modified().unwrap();

        return a_upd.cmp(&b_upd);
    });

    let mut processed = 0;
    let len = files.len();

    if len > 0 {
        // Find start and end indexes for chunk of data that should be proceesed
        // Everything before `end` should be deleted as too old
        let start = files.partition_point(|probe| {
            let time: DateTime<Local> = probe.metadata().unwrap().modified().unwrap().into();
            return time < *delete_from;
        });

        // checking for bounds to not 
        if start < len {
            // Everything after start should not be touched as too new
            let end = start
                + files[start..].partition_point(|probe| {
                    let time: DateTime<Local> = probe.metadata().unwrap().modified().unwrap().into();
                    return time < *archive_from;
                });

            for deleted in &files[0..start] {
                fs::remove_file(deleted.path()).unwrap();
            }

            processed = archive_files(&files[start..end], &dir);
        }
    }

    for sub in list_subdirs(&dir) {
        processed += process_dir(&sub.path(), archive_from, delete_from);
    }

    return processed;
}
