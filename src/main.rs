use termion::{cursor, clear};

fn clear() { println!("{}{}", clear::All, cursor::Goto(1, 1)); }

fn goto(x: u16, y: u16) { println!("{}", cursor::Goto(x, y)); }

/*fn disp_entry(entry: string, size: ){
    println!(
}*/

fn main() {
    //println!("Size is {:?}", terminal_size().unwrap());

    
    let mut rooms: Vec<String> = vec!["Pretty People".to_string(), "Crypto Chat".to_string(), "Free Software Extremists".to_string(), "General".to_string()];
    
    loop {
	let size = termion::terminal_size().unwrap();
	let width = size.0;
	let height = size.1;
	let sep: u16 = width / 3;

	let mut moving_pos = (1, 1);
	
	clear();

	// build sidebar

	for y in 1..height {
	    println!("{}|", cursor::Goto(sep, y));
	}


	
	for room in rooms {
	    println!("{}|{} ", cursor::Goto(moving_pos.0, moving_pos.1), room);
	    moving_pos.1 += 1;
	    for x in 2..sep {
		println!("{}-", cursor::Goto(x, moving_pos.1));
	    }
	    println!("{}|", cursor::Goto(moving_pos.0, moving_pos.1));

	    moving_pos.1 += 1;
	}
	
	break;
    }
}
