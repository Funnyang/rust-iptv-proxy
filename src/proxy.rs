use std::net::{IpAddr, Ipv4Addr, SocketAddrV4};

use actix_web::web::Bytes;
use anyhow::Result;
use async_stream::stream;
use futures_core::stream::Stream;
use futures_util::stream::StreamExt;
use local_ip_address::list_afinet_netifas;
use log::{debug, error, info};
use reqwest::Url;
use retina::client::{PacketItem, Session, SessionOptions};
use rtp_rs::RtpReader;
use tokio::{net::UdpSocket, sync::mpsc};
use tokio_util::bytes::Buf;
use tokio_util::codec::BytesCodec;
use tokio_util::udp::UdpFramed;

fn filter_reordered_seq(seq: &mut u16, next: u16) -> bool {
    let valid = seq.wrapping_add(3000);
    if *seq == 0
        || (valid > *seq && next > *seq && next <= valid)
        || (valid < *seq && (next > *seq || next <= valid))
    {
        *seq = next;
        true
    } else {
        false
    }
}

pub(crate) fn rtsp(url: String, if_name: Option<String>) -> impl Stream<Item = Result<Bytes>> {
    stream! {
        let mut options = SessionOptions::default().follow_redirects(true);
        #[cfg(not(any(target_os = "android", target_os = "fuchsia", target_os = "linux")))]
        if let Some(ref i) = if_name {
            let network_interfaces = list_afinet_netifas()?;
            for (name, ip) in network_interfaces.iter() {
                debug!("{}: {}", name, ip);
                if name == i {
                    options = options.bind(ip.to_string());
                    break;
                }
            }
        }

        #[cfg(any(target_os = "android", target_os = "fuchsia", target_os = "linux"))]
        if let Some(i) = if_name {
            options = options.bind(i);
        }
        debug!("rtsp: {}", url);
        let mut session = match Session::describe(Url::parse(&url)?, options).await {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to create RTSP session: {}", e);
                return;
            }
        };

        info!("RTSP session created with {} streams", session.streams().len());

        for i in 0..session.streams().len() {
            session.setup(i, Default::default()).await?;
        }
        let mut playing = session.play(Default::default()).await?;

        let (tx, mut rx) = mpsc::channel(128);

        tokio::spawn(async move {
            let mut seq = 0u16;
            while let Some(item) = playing.next().await {
                if let Ok(PacketItem::Rtp(stream)) = item {
                    if !filter_reordered_seq(&mut seq, stream.sequence_number()) ||
                        tx.send(stream.into_payload_bytes()).await.is_ok() {
                        continue;
                    }
                }
                break;
            }
        });

        loop {
            if let Some(stream) = rx.recv().await {
                yield Ok(stream);
            } else {
                error!("Connection closed");
                break;
            }
        }
    }
}

pub(crate) fn udp(
    multi_addr: SocketAddrV4,
    if_name: Option<String>,
) -> impl Stream<Item = Result<Bytes>> {
    stream! {
        let socket =  {
            let socket = socket2::Socket::new(
                socket2::Domain::IPV4,
                socket2::Type::DGRAM,
                Some(socket2::Protocol::UDP),
            )?;
            socket.set_reuse_address(true)?;
            #[cfg(not(target_os = "windows"))]
            {
                socket.bind(&multi_addr.into())?;
            }
            #[cfg(target_os = "windows")]
            {
                socket.bind(&SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), multi_addr.port()).into())?;
            }
            UdpSocket::from_std(socket.into())?
        };

        let mut interface = Ipv4Addr::new(0, 0, 0, 0);
        if let Some(ref i) = if_name {
            let network_interfaces = list_afinet_netifas()?;
            if let Some((_, ip)) = network_interfaces.iter()
                .find(|(name, _)| name == i) {
                if let IpAddr::V4(ip) = ip {
                    interface = *ip;
                }
            }
        }

        socket.set_multicast_loop_v4(true)?;

        socket.join_multicast_v4(
            *multi_addr.ip(),
            interface,
        )?;

        info!("Udp proxy joined {}", multi_addr);

        let mut frames = UdpFramed::new(socket, BytesCodec::new());
        let (tx, mut rx) = mpsc::channel(128);

        tokio::spawn(async move {
            let mut seq = 0u16;
            while let Some(item) = frames.next().await {
                if let Ok((bytes, _)) = item {
                    let mut bytes = bytes.freeze();
                    if let Ok(rtp) = RtpReader::new(bytes.as_ref()) {
                        let next = rtp.sequence_number().into();
                        bytes.advance(rtp.payload_offset());
                        if !filter_reordered_seq(&mut seq, next) || tx.send(bytes).await.is_ok() {
                            continue;
                        }
                    }
                }
                frames.get_mut().leave_multicast_v4(
                    *multi_addr.ip(),
                    interface,
                ).ok();
                info!("Udp proxy left {}", multi_addr);
                break;
            }
        });

        loop {
            if let Some(stream) = rx.recv().await {
                yield Ok(stream);
            } else {
                error!("Connection closed");
                break;
            }
        }
    }
}