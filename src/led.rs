use std::net::{SocketAddr, UdpSocket};
use std::{cmp::min, io};

use ndarray::{arr1, Array1, Array2, Axis, Slice};

use crate::config::Config;
use crate::gamma_table::GAMMA_TABLE;

#[derive(Debug)]
struct ESP8266Conn {
    socket: UdpSocket,
    gamma_table: Option<Array1<u8>>,
    address: SocketAddr,
}

static MAX_PIXELS_PER_PACKET: usize = 126;

impl ESP8266Conn {
    /// Create a new ESP8266 connection with a specified ip and gamma correction. The socket throws
    /// an io error if it cannot bind
    pub fn new(config: &Config) -> Result<ESP8266Conn, std::io::Error> {
        Ok(ESP8266Conn {
            socket: UdpSocket::bind("0.0.0.0:0")?,
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
    use std::{net::UdpSocket, thread};

    use ndarray::{Array, Array2, Axis, Slice};
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

        let conn = ESP8266Conn::new(&Config {
            device_ip: String::from("127.0.0.1"),
            device_port: 7777,
            ..Default::default()
        })
        .unwrap();
        let (px, px_prev) = (pixels.clone(), pixels_prev.clone());

        let handle = thread::spawn(move || {
            let recv = UdpSocket::bind("127.0.0.1:7777").unwrap();
            let mut buf = [0; 2048];
            let recv_len = recv.recv(&mut buf).unwrap();

            // reconstruct transmitted buffer
            let cmp_buf: Vec<u8> = px
                .axis_iter(Axis(0))
                .enumerate()
                .filter_map(|(idx, val)| {
                    if val
                        == px_prev
                            .slice_axis(
                                Axis(0),
                                Slice::new(idx as isize, Some(idx as isize + 1), 1),
                            )
                            .into_shape(3)
                            .unwrap()
                    {
                        None
                    } else {
                        Some([&[idx as u8], val.to_slice().unwrap()].concat())
                    }
                })
                .flatten()
                .collect();

            assert_eq!(buf[..recv_len], cmp_buf);
        });

        let bytes_written = conn.update(&mut pixels, &mut pixels_prev).unwrap();
        handle.join().unwrap();
        assert_eq!(bytes_written, num_different * 4);
    }
}
