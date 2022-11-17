use chrono::{DateTime, Datelike, Duration, Local, Timelike};
use clap::Parser;
use std::{
    fs::{self, DirEntry},
    path::PathBuf,
    process::exit,
    time::Instant,
};
use ureq;
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

    /// Proxy in format <user>:<password>@<domain or ip>:<port (optional)>
    #[clap(short, long, required = false)]
    proxy_url: Option<String>,

    /// Token of the bot which is used to send notifications
    #[clap(short, long, required = false)]
    telegram_token: Option<String>,

    /// Id of the chat where to send a notification
    #[clap(short, long, required = false)]
    chat_id: Option<String>,
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

    // Add -1 becuase of partition point algorithm finds index where it's "given date and less",
    // and we need strictly less than gived date
    let archive_from = chrono::offset::Local::now() - Duration::days(args.archive as i64 - 1);
    let archive_from = normalize_date(&archive_from);

    let delete_from = chrono::offset::Local::now() - Duration::days(args.delete as i64);
    let delete_from = normalize_date(&delete_from);

    process_dir(&args.directory, &archive_from, &delete_from);

    send_telegram_notification(&args, now.elapsed());
}

fn send_telegram_notification(args: &Args, elapsed_time: std::time::Duration) {
    if args.telegram_token.is_none() {
        println!("No notificaion send since no telegram token provided.");
        return;
    }

    if args.chat_id.is_none() {
        println!("Cannot send message without chat id provided!");
        return;
    }

    let http = get_http_client(&args.proxy_url);
    if let Err(e) = &http {
        println!("Bad proxy configuration: {e}");
        return;
    }

    let token = args.telegram_token.clone().unwrap();
    let chat_id = args.chat_id.clone().unwrap();
    let http = http.unwrap();

    // TODO: Add ip info
    let text = format!(
        "Произведена очистка логов на `<IP>`\n*Время выполенения*: {:.2}s",
        elapsed_time.as_secs_f32()
    );

    let res = http
        .post(&format!("https://api.telegram.org/bot{token}/sendMessage"))
        .send_json(ureq::json!({
            "chat_id": chat_id,
            "text": text,
            "parse_mode": "markdown"
        }));

    if let Err(e) = res {
        let e = e
            .into_response()
            .unwrap()
            .into_json::<ureq::serde_json::Value>()
            .unwrap();

        eprintln!(
            "Failed to send message to telegram: {}",
            e.get("description").unwrap().to_string()
        );
    }
}

fn get_http_client(proxy_url: &Option<String>) -> Result<ureq::Agent, ureq::Error> {
    let mut builder = ureq::AgentBuilder::new();

    if let Some(url) = proxy_url {
        builder = builder.proxy(ureq::Proxy::new(url)?);
    }

    Ok(builder.build())
}

fn list_dir_files(path: &PathBuf) -> Vec<DirEntry> {
    return fs::read_dir(path)
        .unwrap()
        .map(|v| v.unwrap())
        .filter(|v| v.path().is_file() && !v.file_name().to_string_lossy().ends_with(".tar"))
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

fn archive_files(files: &Vec<&DirEntry>, dir: &PathBuf, date: &DateTime<Local>) {
    let human_readable = date.format("%d-%m-%Y");
    let dest = dir.join(format!(
        "{}_{}.tar",
        dir.file_name().unwrap().to_str().unwrap(),
        human_readable
    ));

    let file = fs::File::create(dest).unwrap();
    let mut tar = TarBuilder::new(file);

    // Move all files to newly created directory
    for v in files {
        tar.append_file(v.file_name(), &mut fs::File::open(v.path()).unwrap())
            .unwrap();
        fs::remove_file(v.path()).unwrap();
    }

    tar.finish().unwrap();
}

fn zip_files(files: &[DirEntry], parent_dir: &PathBuf) {
    let mut current_date: DateTime<Local> = files[0].metadata().unwrap().modified().unwrap().into();
    let mut files_to_archive: Vec<&DirEntry> = vec![];

    for f in files.iter() {
        let date: DateTime<Local> = f.metadata().unwrap().modified().unwrap().into();

        if is_same_day(&date, &current_date) {
            files_to_archive.push(f);
            continue;
        }

        archive_files(&files_to_archive, &parent_dir, &current_date);
        // Reset for the next date
        files_to_archive.clear();
        current_date = date.clone();

        files_to_archive.push(f);
    }

    // Archive last date
    archive_files(&files_to_archive, &parent_dir, &current_date);
}

fn process_dir(dir: &PathBuf, archive_from: &DateTime<Local>, delete_from: &DateTime<Local>) {
    // Sort files by modified time
    let mut files = list_dir_files(dir);
    files.sort_by(|a, b| {
        let a_upd = a.metadata().unwrap().modified().unwrap();
        let b_upd = b.metadata().unwrap().modified().unwrap();

        return a_upd.cmp(&b_upd);
    });

    if files.len() > 0 {
        // Find start and end indexes for chunk of data that should be proceesed
        // Everything before `end` should be deleted as too old
        let start = files.partition_point(|probe| {
            let time: DateTime<Local> = probe.metadata().unwrap().modified().unwrap().into();
            return time < *delete_from;
        });

        // Everything after start should not be touched as too new
        let end = start
            + files[start..].partition_point(|probe| {
                let time: DateTime<Local> = probe.metadata().unwrap().modified().unwrap().into();
                return time < *archive_from;
            });

        for deleted in &files[0..start] {
            fs::remove_file(deleted.path()).unwrap();
        }

        zip_files(&files[start..end], &dir);
    }

    for sub in list_subdirs(&dir) {
        process_dir(&sub.path(), archive_from, delete_from);
    }
}
