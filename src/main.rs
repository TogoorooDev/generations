use termion::{cursor, clear, input::TermRead, raw::IntoRawMode};
use std::io::{stdin, stdout, Write};

fn clear() { println!("{}{}", clear::All, cursor::Goto(1, 1)); }

fn goto(x: u16, y: u16) { println!("{}", cursor::Goto(x, y)); }

/*fn disp_entry(entry: string, size: ){
    println!(
}*/

fn main() {
    let mut _stdout = stdout().into_raw_mode().unwrap();
    let mut rooms = vec!["Pretty People".to_string(), "Crypto Chat".to_string(), "Free Software Extremists".to_string(), "General".to_string()];

    let (width, height) = termion::terminal_size().unwrap();
    let sep: u16 = width / 3;
    clear();
    draw_rooms(height, sep, &rooms);
    draw_bottom(height, width);
    print!("{}", cursor::Goto(1, height-1));
    stdout().flush().unwrap();
    for event in stdin().events() {
        break
    }
}

fn draw_rooms(height: u16, sep: u16, rooms: &[String]) {
    // draw separator bar
    for y in 1..height-1 {
        print!("{}|", cursor::Goto(sep, y));
    }
    let mut y = 1;
    for room in rooms {
        // draw room name
        print!("{}|{}", cursor::Goto(1, y), room);
        // go down and draw a separator line before the next room
        y += 1;
        print!("{}|{}", cursor::Goto(1, y), "-".repeat((sep - 2) as usize));
        y += 1;
    }
}

fn draw_bottom(height: u16, width: u16) {
    print!("{}{}", cursor::Goto(1, height-2), "-".repeat(width as usize));
}
