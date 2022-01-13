use termion::{cursor, clear};

fn clear() { println!("{}{}", clear::All, cursor::Goto(1, 1)); }

fn goto(x: u16, y: u16) { println!("{}", cursor::Goto(x, y)); }

/*fn disp_entry(entry: string, size: ){
    println!(
}*/

fn main() {
    let mut rooms = vec!["Pretty People".to_string(), "Crypto Chat".to_string(), "Free Software Extremists".to_string(), "General".to_string()];

    let (width, height) = termion::terminal_size().unwrap();
    let sep: u16 = width / 3;
    clear();
    draw_rooms(height, sep, &rooms);
}

fn draw_rooms(height: u16, sep: u16, rooms: &[String]) {
    // draw separator bar
    for y in 1..height {
        println!("{}|", cursor::Goto(sep, y));
    }
    let mut moving_pos = (1, 1);
    for room in rooms {
        // draw room name
        println!("{}|{} ", cursor::Goto(moving_pos.0, moving_pos.1), room);
        // go down and draw a separator line before the next room
        moving_pos.1 += 1;
        for x in 2..sep {
            println!("{}-", cursor::Goto(x, moving_pos.1));
        }
        println!("{}|", cursor::Goto(moving_pos.0, moving_pos.1));
        moving_pos.1 += 1;
    }
}
