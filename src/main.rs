#[macro_use]
extern crate lazy_static;
extern crate elefren;
extern crate regex;

use elefren::prelude::*;
use elefren::helpers::toml;
use elefren::Mastodon;
use elefren::entities::event::Event;
use elefren::entities::status::Status;
use elefren::entities::notification::NotificationType;
use elefren::status_builder::StatusBuilder;
use std::error::Error;
use regex::Regex;
use regex::RegexBuilder;
use std::result::Result::Ok;

const YAKI_PATTERN: &str = r"([あアｱ][ひヒﾋ][るルﾙ]|家鴨)[やヤﾔ焼][きキｷ]|ahiruyaki|扒家鸭|3v\.7g";
const REPLACE_CASE: &str = r"[\u200B-\u200D\uFEFF\u3164\s]|(<br\s*/?>\s*)+|(</?p>\s*)+";

fn main() -> Result<(), Box<dyn Error>> {
    let client = Mastodon::from(toml::from_file(".settings.toml")?);

    for event in client.streaming_user()? {
        match event {
            // タイムライン更新イベント
            Event::Update(ref status) => {
                yakuna(&client, status).expect("failed to send response");
            },
            // 通知
            Event::Notification(ref notification) => {
                match notification.notification_type {
                    NotificationType::Mention => {
                        yakuna(&client, &notification.status.clone()?).expect("failed to send response");
                    },
                    _ => ()
                }
            }
            _ => ()
        }
    }
    Ok(())
}

fn yakuna(client: &Mastodon, ref status: &Status) -> Result<Status, Box<dyn Error>> {
    if !status.is_burning() {
        return Result::Err(Box::from("not yaki"));
    }
    status.reply(client, "焼くな")
}

fn is_need_burning(text: &str) -> bool {
    lazy_static! {
        static ref YAKUNA_REGEX: Regex = RegexBuilder::new(YAKI_PATTERN).case_insensitive(true).build().unwrap();
        static ref REPLACE_REGEX: Regex = Regex::new(REPLACE_CASE).unwrap();
    }
    let replaced = REPLACE_REGEX.replace_all(text, "");
    YAKUNA_REGEX.is_match(&replaced)
}

trait StatusExt {
    fn is_burning(&self) -> bool;
    fn reply(&self, client: &Mastodon, message: &str) -> Result<Status, Box<dyn Error>>;
}

impl StatusExt for Status {
    fn is_burning(&self) -> bool {
        is_need_burning(&self.content)
    }

    fn reply(&self, client: &Mastodon, message: &str) -> Result<Status, Box<dyn Error>> {
        let status = StatusBuilder::new()
            .status(format!("@{} {}", self.account.acct.clone(), message))
            .visibility(self.visibility)
            .in_reply_to(self.id.clone())
            .build()?;

        client.new_status(status).map_err(|e| Box::from(e))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_burn() {
        let test_cases: [&str; 11] = [
            "あひる焼き",
            "家鴨焼き",
            "あひるやき",
            "ahiruyaki",
            "扒家鸭",
            "3v.7g",
            "あひル焼ｷ",
            "アﾋるやキ",
            "家鴨やｷ",
            "AhiRuYaki",
            "あㅤひる焼き",
        ];

        for case in test_cases.iter() {
            assert!(is_need_burning(case));
        }
    }

    #[test]
    fn should_remove_space() {
        let test_cases = "あㅤひる焼き";

        let replaced = Regex::new(REPLACE_CASE).unwrap().replace_all(test_cases, "");
        assert_eq!(replaced, "あひる焼き")
    }

    #[test]
    fn should_remove_new_line_and_burn() {
        let test_cases: [&str; 2] = [
            "日直<br> あ<br> ひ<br> る<br> 焼<br>　き",
            "<p>日直</p><p>あ</p><p>ひ</p><p>る</p><p>焼き</p>",
        ];
        let expected = "日直あひる焼き";

        lazy_static! {
            static ref YAKUNA_REGEX: Regex = Regex::new(YAKI_PATTERN).unwrap();
            static ref REPLACE_REGEX: Regex = Regex::new(REPLACE_CASE).unwrap();
        }

        for case in test_cases.iter() {
            let replaced = REPLACE_REGEX.replace_all(case, "");
            assert_eq!(&replaced, expected);
            assert!(YAKUNA_REGEX.is_match(&replaced));
        }
    }

    #[test]
    fn should_not_burn() {
        let test_cases: [&str; 2] = [
            "ahiru焼き",
            "焼きあひる",
        ];

        for case in test_cases.iter() {
            assert!(!is_need_burning(case));
        }
    }
}
