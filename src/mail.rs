use mail_parser::DateTime;
use redis::RedisError;
use tracing::{error, trace};

#[derive(Debug, Clone)]
pub struct Mail {
    pub con: redis::aio::MultiplexedConnection,
}

impl Mail {
    pub async fn new(url: &str) -> Mail {
        let client = redis::Client::open(url).expect("Failed to open Redis client.");
        // This shit isn't enough tho, we have to manually test later.
        client
            .get_connection()
            .expect("Failed to connect to Redis.");
        let con = client.get_multiplexed_async_connection().await.unwrap();
        Mail { con }
    }
    pub async fn db_save_users_with_mail(&self, users: &Vec<String>, mail_id: &String) -> Result<(), RedisError> {
        for username in users {
            let save_db_cmd_1 = redis::Cmd::lpush(format!("user:{}", username), format!("mail:{}", mail_id));
            let con_1 = &mut self.con.clone();
            match tokio::try_join!(
                save_db_cmd_1.exec_async(con_1),
            ) {
                Ok(_) => {
                    trace!("Saved user to database: {}", mail_id);
                }
                Err(e) => {
                    error!("Failed to save user to database: {}", e);
                    return Err(e);
                }
            }
        }
        return Ok(());
    }
    pub async fn db_save_mail_metadata(&self, id: &String, has_body_text: bool, has_body_html: bool, attachments_len: usize, subject: &String, sender: &String, body_text_preview: &String, timestamp: &DateTime) -> Result<(), RedisError>{
        // This shit looks dumb af but okay.
        let save_db_cmd_1 = redis::Cmd::hset_multiple(format!("mail:{}", id), &[
            ("has_body_text", has_body_text),
            ("has_body_html", has_body_html),
        ]);
        let save_db_cmd_2 =
            redis::Cmd::hset(format!("mail:{}", id), "attachments", attachments_len);
        let save_db_cmd_3 = redis::Cmd::hset(
            format!("mail:{}", id),
            "timestamp",
            timestamp.to_timestamp(),
        );
        let save_db_cmd_4 = redis::Cmd::hset_multiple(format!("mail:{}", id), &[
            ("subject", subject),
            ("sender", sender),
            ("body_text_preview", body_text_preview),
        ]);
        let con_1 = &mut self.con.clone();
        let con_2 = &mut self.con.clone();
        let con_3 = &mut self.con.clone();
        let con_4 = &mut self.con.clone();
        match tokio::try_join!(
            save_db_cmd_1.exec_async(con_1),
            save_db_cmd_2.exec_async(con_2),
            save_db_cmd_3.exec_async(con_3),
            save_db_cmd_4.exec_async(con_4),
        ) {
            Ok(_) => {
                trace!("Saved mail to database: {}", id);
                return Ok(());
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}

