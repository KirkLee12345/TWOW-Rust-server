use std::fs;
use rand::Rng;
use crate::lib::email;
use email::EmailSender;
use crate::lib::server::FROM_EMAIL;

fn generate_yzm() -> String {
    let mut rng = rand::thread_rng();
    let code: u32 = rng.gen_range(100000..1000000);
    format!("{:06}", code)
}

pub fn send_yzm(thread_index: usize, email: String) -> String {
    let email_sender = EmailSender::new(
        FROM_EMAIL.to_string(),
        fs::read_to_string("email.key").expect("错误: 无法读取 email.key 文件，请确保该文件存在于程序运行目录。")
    );
    let yzm = generate_yzm();

    let subject = "兵者账号注册-邮箱验证";
    let body = format!("感谢您注册兵者账号!您本次注册的验证码为 {yzm} ,请尽快完成注册!");

    match email_sender.send_email(&email, subject, &body) {
        Ok(_) => println!("[{thread_index}] 验证码 {yzm} 已发送至: {email}"),
        Err(e) => eprintln!("[{thread_index}] 发送验证码失败: {e}"),
    }

    yzm
}
