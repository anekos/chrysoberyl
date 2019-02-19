
use std::sync::mpsc::Sender;
use std::thread::spawn;

use rusoto_core::Region;
use rusoto_rekognition::{DetectFacesRequest, DetectFacesResponse, Image, Rekognition, RekognitionClient};

use crate::operation::CherenkovParameter;
use crate::operation::Operation;
use crate::size::Coord;




pub fn detect_eyes(app_tx: Sender<Operation>, parameter: CherenkovParameter, image: Vec<u8>) {
    let cli = RekognitionClient::new(Region::default());

    let image = Image { bytes: Some(image), s3_object: None };
    let request = DetectFacesRequest {
        attributes: None,
        image,
    };

    spawn(move || {
        match cli.detect_faces(request).sync() {
            Ok(response) => {
                let eyes = extract_eyes(response);
                if eyes.is_empty() {
                    puts_error!("Eyes not found", "at" => "cherenkov/detect_eyes");
                    app_tx.send(Operation::Message(Some(o!("Eyes not found")), false)).unwrap();
                } else {
                    for eye in &eyes {
                        let mut parameter = parameter.clone();
                        parameter.x = Some(eye.x);
                        parameter.y = Some(eye.y);
                        app_tx.send(Operation::Cherenkov(parameter)).unwrap();
                    }
                    app_tx.send(Operation::Message(Some(format!("{} eyes were detected", eyes.len())), false)).unwrap();
                }
            },
            Err(err) => {
                puts_error!(err, "at" => "cherenkov/detect_eyes");
                app_tx.send(Operation::Message(Some(s!(err)), false)).unwrap();
            },
        }
    });
}


fn extract_eyes(res: DetectFacesResponse) -> Vec<Coord> {
    if_let_some!(details = res.face_details, vec![]);
    let mut result = vec![];

    for landmarks in details.iter().flat_map(|it| &it.landmarks) {
        for landmark in landmarks {
            match landmark.type_.as_ref().map(String::as_ref) {
                Some("eyeRight") | Some("eyeLeft") =>
                    if let (Some(x), Some(y)) = (landmark.x, landmark.y) {
                        result.push(Coord { x: f64!(x), y: f64!(y) })
                    },
                _ => (),
            }
        }
    }

    result
}
