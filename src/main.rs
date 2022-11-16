use std::{fs, path::PathBuf, process::exit, time::Instant};

use chrono::{DateTime, Datelike, Duration, Local, Timelike};
use clap::Parser;
use walkdir::{DirEntry, WalkDir};
// se zip_archive::{Archiver, Format};
use tar::Builder as TarBuilder;

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

fn list_files_recursively(path: &PathBuf) -> Vec<DirEntry> {
    return WalkDir::new(path)
        .into_iter()
        .filter(|v| v.is_ok())
        .map(|v| v.unwrap())
        .filter(|v| {
            v.metadata().unwrap().is_file() && !v.file_name().to_string_lossy().ends_with(".tar")
        })
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

// TODO: Keep structure of folders inside the given directy, save it even inside archive
fn is_same_day(a: &DateTime<Local>, b: &DateTime<Local>) -> bool {
    return a.year() == b.year() && a.month() == b.month() && a.day() == b.day();
}

fn zip_files(files: &[DirEntry], dir: &PathBuf) {
    if files.len() == 0 {
        return;
    }

    let mut current_date: DateTime<Local> = files[0]
        .clone()
        .metadata()
        .unwrap()
        .modified()
        .unwrap()
        .into();

    let mut files_to_archive: Vec<&DirEntry> = vec![];
    // let mut archiver = Archiver::new();

    // archiver.set_format(Format::Zip);
    // archiver.set_thread_count(4);

    for (i, f) in files.iter().enumerate() {
        let date: DateTime<Local> = f.metadata().unwrap().modified().unwrap().into();

        if is_same_day(&date, &current_date) {
            files_to_archive.push(f);

            if i != files.len() - 1 {
                continue;
            }
        }

        let human_readable = current_date.format("%d-%m-%Y");
        let dest = dir.join(format!(
            "{}_{}.tar",
            dir.file_name().unwrap().to_str().unwrap(),
            human_readable
        ));

        let file = fs::File::create(dest).unwrap();
        let mut tar = TarBuilder::new(file);

        // Move all files to newly created directory
        for v in &files_to_archive {
            tar.append_file(v.file_name(), &mut fs::File::open(v.path()).unwrap())
                .unwrap();
            fs::remove_file(v.path()).unwrap();
        }

        tar.finish().unwrap();

        // Reset for the next date
        files_to_archive.clear();

        current_date = date.clone();

        // Add current file (in case it's not last and it was not added at line 70 bc of different date)
        if i != files.len() - 1 {
            files_to_archive.push(f);
        }
    }
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

    // Sort files by modified time
    let mut files = list_files_recursively(&args.directory);
    files.sort_by(|a, b| {
        let a_upd = a.metadata().unwrap().modified().unwrap();
        let b_upd = b.metadata().unwrap().modified().unwrap();

        return a_upd.cmp(&b_upd);
    });

    // Add -1 becuase of partition point algorithm finds index where it's "given date and less",
    // and we need strictly less than gived date
    let archive_from = chrono::offset::Local::now() - Duration::days(args.archive as i64 - 1);
    let archive_from = normalize_date(&archive_from);

    let delete_from = chrono::offset::Local::now() - Duration::days(args.delete as i64);
    let delete_from = normalize_date(&delete_from);

    // Find start and end indexes for chunk of data that should be proceesed
    // Everything before `end` should be deleted as too old
    let start = files.partition_point(|probe| {
        let time: DateTime<Local> = probe.metadata().unwrap().modified().unwrap().into();
        return time < delete_from;
    });

    // Everything after start should not be touched as too new
    let end = start
        + files[start..].partition_point(|probe| {
            let time: DateTime<Local> = probe.metadata().unwrap().modified().unwrap().into();
            return time < archive_from;
        });

    // for i in &files[start..end] {
    //     println!("{:#?}", DateTime::<Local>::from(i.metadata().unwrap().modified().unwrap()).to_rfc2822());
    // }

    for deleted in &files[0..start] {
        fs::remove_file(deleted.path()).unwrap();
    }

    zip_files(&files[start..end], &args.directory);

    println!("Done in {}ms", now.elapsed().as_millis());
}
