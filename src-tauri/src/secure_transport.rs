use std::{
    fmt,
    sync::{Arc, Mutex},
    time::Duration,
};

use base64::{engine::general_purpose::STANDARD, Engine};
use rustls::{
    client::danger::{ServerCertVerified, ServerCertVerifier},
    crypto::{verify_tls12_signature, verify_tls13_signature, WebPkiSupportedAlgorithms},
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime},
    server::danger::{ClientCertVerified, ClientCertVerifier},
    CertificateError, ClientConfig, DigitallySignedStruct, DistinguishedName, Error, ServerConfig,
    SignatureScheme,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::net::TcpStream;
use tokio_rustls::{client, server, TlsAcceptor, TlsConnector};

use crate::error::{AppError, AppResult};

const KEYRING_SERVICE: &str = "CopyShare";
const KEYRING_USER: &str = "desktop-tls-identity";
const TLS_TIMEOUT: Duration = Duration::from_secs(5);
const WS_ALPN: &[u8] = b"copyshare-ws";
const FILE_ALPN: &[u8] = b"copyshare-file";

static IDENTITY: Mutex<Option<Arc<Identity>>> = Mutex::new(None);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protocol {
    WebSocket,
    File,
}

#[derive(Serialize, Deserialize)]
struct StoredIdentity {
    certificate: String,
    private_key: String,
}

struct Identity {
    server: Arc<ServerConfig>,
    websocket_client: Arc<ClientConfig>,
    file_client: Arc<ClientConfig>,
    fingerprint: String,
}

struct PeerVerifier {
    algorithms: WebPkiSupportedAlgorithms,
}

impl fmt::Debug for PeerVerifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PeerVerifier")
    }
}

impl PeerVerifier {
    fn new() -> Self {
        Self {
            algorithms: rustls::crypto::aws_lc_rs::default_provider()
                .signature_verification_algorithms,
        }
    }

    fn check_certificate(&self, certificate: &CertificateDer<'_>) -> Result<(), Error> {
        if certificate.is_empty() {
            Err(Error::InvalidCertificate(CertificateError::BadEncoding))
        } else {
            Ok(())
        }
    }

    fn verify_tls12(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, Error> {
        verify_tls12_signature(message, certificate, signature, &self.algorithms)
    }

    fn verify_tls13(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, Error> {
        verify_tls13_signature(message, certificate, signature, &self.algorithms)
    }
}

impl ServerCertVerifier for PeerVerifier {
    fn verify_server_cert(
        &self,
        certificate: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        self.check_certificate(certificate)?;
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, Error> {
        self.verify_tls12(message, certificate, signature)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, Error> {
        self.verify_tls13(message, certificate, signature)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algorithms.supported_schemes()
    }
}

impl ClientCertVerifier for PeerVerifier {
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        certificate: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: UnixTime,
    ) -> Result<ClientCertVerified, Error> {
        self.check_certificate(certificate)?;
        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, Error> {
        self.verify_tls12(message, certificate, signature)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, Error> {
        self.verify_tls13(message, certificate, signature)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algorithms.supported_schemes()
    }
}

fn identity() -> AppResult<Arc<Identity>> {
    let mut cached = IDENTITY.lock().unwrap_or_else(|error| error.into_inner());
    if let Some(identity) = cached.as_ref() {
        return Ok(identity.clone());
    }

    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|error| AppError::InvalidInput(format!("无法访问系统凭据存储：{error}")))?;
    let stored = match entry.get_password() {
        Ok(encoded) => serde_json::from_str::<StoredIdentity>(&encoded)
            .map_err(|error| AppError::InvalidInput(format!("设备加密身份已损坏：{error}")))?,
        Err(keyring::Error::NoEntry) => {
            let generated = generate_identity()?;
            entry
                .set_password(&serde_json::to_string(&generated)?)
                .map_err(|error| {
                    AppError::InvalidInput(format!("无法保存设备加密身份：{error}"))
                })?;
            generated
        }
        Err(error) => {
            return Err(AppError::InvalidInput(format!(
                "无法读取设备加密身份：{error}"
            )));
        }
    };
    let identity = Arc::new(build_identity(&stored)?);
    *cached = Some(identity.clone());
    Ok(identity)
}

fn generate_identity() -> AppResult<StoredIdentity> {
    let generated = rcgen::generate_simple_self_signed(vec!["copyshare.local".to_string()])
        .map_err(|error| AppError::InvalidInput(format!("无法生成设备加密身份：{error}")))?;
    Ok(StoredIdentity {
        certificate: STANDARD.encode(generated.cert.der()),
        private_key: STANDARD.encode(generated.signing_key.serialize_der()),
    })
}

fn build_identity(stored: &StoredIdentity) -> AppResult<Identity> {
    let certificate = STANDARD
        .decode(&stored.certificate)
        .map_err(|error| AppError::InvalidInput(format!("设备证书无效：{error}")))?;
    let private_key = STANDARD
        .decode(&stored.private_key)
        .map_err(|error| AppError::InvalidInput(format!("设备私钥无效：{error}")))?;
    let fingerprint = fingerprint(&certificate);
    let cert = CertificateDer::from(certificate);
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(private_key));
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let mut server = ServerConfig::builder_with_provider(provider.clone())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|error| AppError::InvalidInput(format!("TLS 配置失败：{error}")))?
        .with_client_cert_verifier(Arc::new(PeerVerifier::new()))
        .with_single_cert(vec![cert.clone()], key.clone_key())
        .map_err(|error| AppError::InvalidInput(format!("设备证书无效：{error}")))?;
    server.alpn_protocols = vec![WS_ALPN.to_vec(), FILE_ALPN.to_vec()];
    let make_client = |protocol: &[u8]| -> AppResult<Arc<ClientConfig>> {
        let mut client = ClientConfig::builder_with_provider(provider.clone())
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|error| AppError::InvalidInput(format!("TLS 配置失败：{error}")))?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(PeerVerifier::new()))
            .with_client_auth_cert(vec![cert.clone()], key.clone_key())
            .map_err(|error| AppError::InvalidInput(format!("设备证书无效：{error}")))?;
        client.alpn_protocols = vec![protocol.to_vec()];
        Ok(Arc::new(client))
    };
    Ok(Identity {
        server: Arc::new(server),
        websocket_client: make_client(WS_ALPN)?,
        file_client: make_client(FILE_ALPN)?,
        fingerprint,
    })
}

fn fingerprint(certificate: &[u8]) -> String {
    format!("{:x}", Sha256::digest(certificate))
}

fn peer_fingerprint(certificates: Option<&[CertificateDer<'static>]>) -> AppResult<String> {
    certificates
        .and_then(|certificates| certificates.first())
        .map(|certificate| fingerprint(certificate.as_ref()))
        .ok_or_else(|| AppError::InvalidInput("设备未提供加密身份证书".to_string()))
}

pub fn local_fingerprint() -> AppResult<String> {
    Ok(identity()?.fingerprint.clone())
}

pub fn pairing_code(peer: &str) -> AppResult<String> {
    let own = local_fingerprint()?;
    Ok(pairing_code_for(&own, peer))
}

fn pairing_code_for(own: &str, peer: &str) -> String {
    let (first, second) = if own <= peer {
        (own, peer)
    } else {
        (peer, own)
    };
    let hash = format!(
        "{:X}",
        Sha256::digest(format!("{first}:{second}").as_bytes())
    );
    hash[..16]
        .as_bytes()
        .chunks(4)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join("-")
}

pub async fn accept(
    stream: TcpStream,
) -> AppResult<(server::TlsStream<TcpStream>, Protocol, String)> {
    let acceptor = TlsAcceptor::from(identity()?.server.clone());
    let stream = tokio::time::timeout(TLS_TIMEOUT, acceptor.accept(stream))
        .await
        .map_err(|_| AppError::ConnectionTimeout("设备 TLS 握手超时".to_string()))?
        .map_err(|error| AppError::InvalidInput(format!("设备 TLS 握手失败：{error}")))?;
    let protocol = match stream.get_ref().1.alpn_protocol() {
        Some(WS_ALPN) => Protocol::WebSocket,
        Some(FILE_ALPN) => Protocol::File,
        _ => return Err(AppError::InvalidInput("设备协议协商失败".to_string())),
    };
    let fingerprint = peer_fingerprint(stream.get_ref().1.peer_certificates())?;
    Ok((stream, protocol, fingerprint))
}

pub async fn connect(
    host: &str,
    port: u16,
    protocol: Protocol,
    expected_fingerprint: Option<&str>,
) -> AppResult<(client::TlsStream<TcpStream>, String)> {
    let identity = identity()?;
    let config = match protocol {
        Protocol::WebSocket => identity.websocket_client.clone(),
        Protocol::File => identity.file_client.clone(),
    };
    let stream = tokio::time::timeout(TLS_TIMEOUT, TcpStream::connect((host, port)))
        .await
        .map_err(|_| AppError::ConnectionTimeout("设备连接超时".to_string()))??;
    let server_name = ServerName::try_from("copyshare.local")
        .map_err(|_| AppError::InvalidInput("设备 TLS 名称无效".to_string()))?;
    let stream = tokio::time::timeout(
        TLS_TIMEOUT,
        TlsConnector::from(config).connect(server_name, stream),
    )
    .await
    .map_err(|_| AppError::ConnectionTimeout("设备 TLS 握手超时".to_string()))?
    .map_err(|error| AppError::InvalidInput(format!("设备 TLS 握手失败：{error}")))?;
    let fingerprint = peer_fingerprint(stream.get_ref().1.peer_certificates())?;
    if expected_fingerprint.is_some_and(|expected| expected != fingerprint) {
        return Err(AppError::InvalidInput(
            "设备加密身份已变化，请重新配对".to_string(),
        ));
    }
    Ok((stream, fingerprint))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    #[test]
    fn identity_and_pairing_code_are_stable() {
        let stored = generate_identity().unwrap();
        let identity = build_identity(&stored).unwrap();
        assert_eq!(identity.fingerprint.len(), 64);
        assert_eq!(
            build_identity(&stored).unwrap().fingerprint,
            identity.fingerprint
        );
        let other = build_identity(&generate_identity().unwrap()).unwrap();
        assert_eq!(
            pairing_code_for(&identity.fingerprint, &other.fingerprint),
            pairing_code_for(&other.fingerprint, &identity.fingerprint)
        );
        let impostor = build_identity(&generate_identity().unwrap()).unwrap();
        assert_ne!(
            pairing_code_for(&identity.fingerprint, &other.fingerprint),
            pairing_code_for(&identity.fingerprint, &impostor.fingerprint)
        );
    }

    #[tokio::test]
    async fn tls_authenticates_both_devices_and_encrypts_websocket_bytes() {
        let server = build_identity(&generate_identity().unwrap()).unwrap();
        let client = build_identity(&generate_identity().unwrap()).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let client_fingerprint = client.fingerprint.clone();
        let server_fingerprint = server.fingerprint.clone();
        let server_task = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut stream = TlsAcceptor::from(server.server)
                .accept(socket)
                .await
                .unwrap();
            assert_eq!(stream.get_ref().1.alpn_protocol(), Some(WS_ALPN));
            assert_eq!(
                peer_fingerprint(stream.get_ref().1.peer_certificates()).unwrap(),
                client_fingerprint
            );
            let mut received = [0u8; 6];
            stream.read_exact(&mut received).await.unwrap();
            assert_eq!(&received, b"secret");
            stream.write_all(b"ok").await.unwrap();
        });
        let socket = TcpStream::connect(address).await.unwrap();
        let mut stream = TlsConnector::from(client.websocket_client)
            .connect(ServerName::try_from("copyshare.local").unwrap(), socket)
            .await
            .unwrap();
        assert_eq!(
            peer_fingerprint(stream.get_ref().1.peer_certificates()).unwrap(),
            server_fingerprint
        );
        stream.write_all(b"secret").await.unwrap();
        let mut received = [0u8; 2];
        stream.read_exact(&mut received).await.unwrap();
        assert_eq!(&received, b"ok");
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn tls_file_channel_is_separate_from_websocket_channel() {
        let server = build_identity(&generate_identity().unwrap()).unwrap();
        let client = build_identity(&generate_identity().unwrap()).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server_task = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let stream = TlsAcceptor::from(server.server)
                .accept(socket)
                .await
                .unwrap();
            assert_eq!(stream.get_ref().1.alpn_protocol(), Some(FILE_ALPN));
        });
        let socket = TcpStream::connect(address).await.unwrap();
        let stream = TlsConnector::from(client.file_client)
            .connect(ServerName::try_from("copyshare.local").unwrap(), socket)
            .await
            .unwrap();
        assert_eq!(stream.get_ref().1.alpn_protocol(), Some(FILE_ALPN));
        server_task.await.unwrap();
    }
}
