use super::SqlBrowser;
use async_std::{
    io,
    net::{self, ToSocketAddrs},
};
use async_trait::async_trait;
use futures::TryFutureExt;
use std::time;

#[async_trait]
impl SqlBrowser for net::TcpStream {
    /// This method can be used to connect to SQL Server named instances
    /// when on a Windows platform with the `sql-browser-async-std` feature
    /// enabled. Please see the crate examples for more detailed examples.
    async fn connect_named(builder: &crate::client::Config) -> crate::Result<Self> {
        let addrs = builder.get_addr().to_socket_addrs().await?;

        for mut addr in addrs {
            dbg!(&addr);
            if let Some(ref instance_name) = builder.instance_name {
                // First resolve the instance to a port via the
                // SSRP protocol/MS-SQLR protocol [1]
                // [1] https://msdn.microsoft.com/en-us/library/cc219703.aspx

                let local_bind: std::net::SocketAddr = if addr.is_ipv4() {
                    "0.0.0.0:0".parse().unwrap()
                } else {
                    "[::]:0".parse().unwrap()
                };

                let msg = [&[4u8], instance_name.as_bytes()].concat();
                let mut buf = vec![0u8; 4096];

                let socket = net::UdpSocket::bind(&local_bind).await?;
                socket.send_to(&msg, &addr).await?;

                let timeout = time::Duration::from_millis(1000);

                let len = io::timeout(timeout, socket.recv(&mut buf))
                    .map_err(|_| {
                        crate::error::Error::Conversion(
                            format!(
                                "SQL browser timeout during resolving instance {}",
                                instance_name
                            )
                            .into(),
                        )
                    })
                    .await?;

                let port = super::get_port_from_sql_browser_reply(buf, len, instance_name)?;
                addr.set_port(port);
            };
            dbg!(&addr);

            if let Ok(stream) = net::TcpStream::connect(addr).await {
                stream.set_nodelay(true)?;
                return Ok(stream);
            }
        }

        Err(io::Error::new(io::ErrorKind::NotFound, "Could not resolve server host.").into())
    }
}
