use std::process::Command;

fn main() {
    let a = Command::new("D:\\rust.etc\\EchoTag\\a.bat").output().unwrap();
    assert_eq!("hello\r\n", String::from_utf8(a.stdout).unwrap());
}
