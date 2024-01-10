use std::net::{SocketAddr, UdpSocket};
use std::{cmp::min, io};

use ndarray::{arr1, Array1, Array2, Axis, Slice};

use crate::config::Config;

#[derive(Debug)]
struct ESP8266Conn {
    socket: UdpSocket,
    gamma_table: Option<Array1<u8>>,
    address: SocketAddr,
}

static GAMMA_TABLE: &[u8] = &[
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4,
    4, 4, 5, 5, 5, 5, 6, 6, 6, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10, 11, 11, 11, 12, 12, 13, 13, 14,
    14, 15, 15, 16, 16, 17, 17, 18, 18, 19, 19, 20, 20, 21, 21, 22, 23, 23, 24, 24, 25, 26, 26, 27,
    28, 28, 29, 30, 30, 31, 32, 32, 33, 34, 35, 35, 36, 37, 38, 38, 39, 40, 41, 42, 42, 43, 44, 45,
    46, 47, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67,
    68, 69, 70, 71, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 84, 85, 86, 87, 88, 89, 91, 92, 93, 94,
    95, 97, 98, 99, 100, 102, 103, 104, 105, 107, 108, 109, 111, 112, 113, 115, 116, 117, 119, 120,
    121, 123, 124, 126, 127, 128, 130, 131, 133, 134, 136, 137, 139, 140, 142, 143, 145, 146, 148,
    149, 151, 152, 154, 155, 157, 158, 160, 162, 163, 165, 166, 168, 170, 171, 173, 175, 176, 178,
    180, 181, 183, 185, 186, 188, 190, 192, 193, 195, 197, 199, 200, 202, 204, 206, 207, 209, 211,
    213, 215, 217, 218, 220, 222, 224, 226, 228, 230, 232, 233, 235, 237, 239, 241, 243, 245, 247,
    249, 251, 253, 255,
];

static MAX_PIXELS_PER_PACKET: usize = 126;

impl ESP8266Conn {
    /// Create a new ESP8266 connection with a specified ip and gamma correction. The socket throws
    /// an io error if it cannot bind
    pub fn new(config: &Config) -> Result<ESP8266Conn, std::io::Error> {
        Ok(ESP8266Conn {
            socket: UdpSocket::bind(format!("127.0.0.1:{}", config.device_port))?,
            address: format!("{}:{}", config.device_ip, config.device_port)
                .parse()
                .expect("Address is not formatted correctly"),
            gamma_table: match config.software_gamma_correction {
                true => Some(arr1(GAMMA_TABLE)),
                false => None,
            },
        })
    }

    /// Sends UDP packets to ESP8266 to update LED strip values

    /// The ESP8266 will receive and decode the packets to determine what values
    /// to display on the LED strip. The communication protocol supports LED strips
    /// with a maximum of 256 LEDs.

    /// The packet encoding scheme is:
    ///     |i|r|g|b|
    /// where
    ///     i (0 to 255): Index of LED to change (zero-based)
    ///     r (0 to 255): Red value of LED
    ///     g (0 to 255): Green value of LED
    ///     b (0 to 255): Blue value of LED
    pub fn update(
        &self,
        pixels: &mut Array2<u8>,
        pixels_prev: &mut Array2<u8>,
    ) -> Result<usize, io::Error> {
        // if the gamma table exists, map it to pixel array
        if let Some(gamma) = &self.gamma_table {
            pixels.map_inplace(|x| *x = gamma[*x as usize]);
        }

        let send_buffer = self.create_send_buffer(pixels, pixels_prev);
        let mut sent = 0;

        for packet in 0..send_buffer.len() / 4 / MAX_PIXELS_PER_PACKET {
            let (packet_start, packet_end) = (
                packet * MAX_PIXELS_PER_PACKET * 4,
                min(send_buffer.len(), (packet + 1) * MAX_PIXELS_PER_PACKET * 4),
            );
            sent += self
                .socket
                .send_to(&send_buffer[packet_start..packet_end], self.address)?;
        }

        Ok(sent)
    }

    // construct the flat buffer of (i, r, g, b) indices
    fn create_send_buffer(&self, pixels: &Array2<u8>, pixels_prev: &Array2<u8>) -> Vec<u8> {
        pixels
            .axis_iter(Axis(0))
            .enumerate()
            .filter_map(|(idx, val)| {
                if val
                    == pixels_prev
                        .slice_axis(Axis(0), Slice::new(idx as isize, Some(idx as isize + 1), 1))
                        .into_shape(3)
                        .unwrap()
                {
                    None
                } else {
                    Some([&[idx as u8], val.to_slice().unwrap()].concat())
                }
            })
            .flatten()
            .collect()
    }
}

#[cfg(test)]
mod test {
    use std::net::UdpSocket;

    use ndarray::{Array, Array2};
    use ndarray_rand::{rand_distr::Uniform, RandomExt};

    use crate::config::Config;

    use super::ESP8266Conn;

    #[test]
    fn test_create_new_from_local_host() {
        ESP8266Conn::new(&Config::default()).unwrap();
    }

    #[test]
    fn test_create_send_buffer() {
        let num_different = 15;
        // create some buffers that are duplicates of one another and modify the pixels in one assert that the length of the send buffer is what we expect
        let pixels_prev: Array2<u8> = Array::random((255, 3), Uniform::new(0., 255.))
            .to_owned()
            .map(|x| *x as u8);
        let mut pixels = pixels_prev.clone();
        pixels
            .slice_mut(ndarray::s![10..10 + num_different, ..])
            .map_mut(|x| *x += 1);

        let conn = ESP8266Conn::new(&Config::default()).unwrap();
        assert_eq!(
            conn.create_send_buffer(&pixels, &pixels_prev).len(),
            num_different * 4
        );
    }

    #[test]
    fn test_update() {
        let num_different = 15;
        // create some buffers that are duplicates of one another and modify the pixels in one assert that the length of the send buffer is what we expect
        let mut pixels_prev: Array2<u8> = Array::random((255, 3), Uniform::new(0., 255.))
            .to_owned()
            .map(|x| *x as u8);
        let mut pixels = pixels_prev.clone();
        pixels
            .slice_mut(ndarray::s![10..10 + num_different, ..])
            .map_mut(|x| *x += 1);
        let mut cfg = Config::default();
        cfg.device_ip = "127.0.0.1".to_string();
        let receiver = UdpSocket::bind("127.0.0.1:7777").unwrap();

        let conn = ESP8266Conn::new(&Config::default()).unwrap();
        conn.update(&mut pixels, &mut pixels_prev).unwrap();
    }
}
