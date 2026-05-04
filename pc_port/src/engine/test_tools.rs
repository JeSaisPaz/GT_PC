use super::model::{load_car_model, load_course, load_car_texture, load_course_texture};

pub fn test_3ldm_parsing() {
    println!("\n=== Testing 3LDM Parser ===");

    println!("\n1. Testing car model loading (00010001):");
    match load_car_model(0x00010001) {
        Ok(car) => {
            println!("   SUCCESS: {} vertices, {} triangles, center = ({:.2}, {:.2}, {:.2})",
                car.vertices.len(), car.triangles.len(), car.center.0, car.center.1, car.center.2);
            if !car.vertices.is_empty() {
                println!("   Sample vertices:");
                for i in 0..3.min(car.vertices.len()) {
                    let (x, y, z) = car.vertices[i];
                    println!("     v[{}] = ({:.3}, {:.3}, {:.3})", i, x, y, z);
                }
            }
        }
        Err(e) => println!("   ERROR: {}", e)
    }

    println!("\n2. Testing car texture loading (00010001):");
    match load_car_texture(0x00010001) {
        Some(tex) => println!("   SUCCESS: {}x{} texture, {} bytes RGBA", tex.width, tex.height, tex.rgba.len()),
        None => println!("   No embedded texture found"),
    }

    println!("\n3. Testing course texture loading (race.txs):");
    match load_course_texture() {
        Some(tex) => println!("   SUCCESS: {}x{} texture, {} bytes RGBA", tex.width, tex.height, tex.rgba.len()),
        None => println!("   Failed to load race.txs"),
    }

    println!("\n4. Testing course loading (c001):");
    match load_course(1) {
        Ok(track) => {
            println!("   SUCCESS: {} vertices, {} triangles, center = ({:.2}, {:.2}, {:.2})",
                track.vertices.len(), track.triangles.len(), track.center.0, track.center.1, track.center.2);
        }
        Err(e) => println!("   ERROR: {}", e)
    }

    println!("\n=== 3LDM Parser Test Complete ===\n");
}