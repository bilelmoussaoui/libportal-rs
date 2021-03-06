//! # Examples
//!
//! ```rust,no_run
//! use ashpd::desktop::camera::CameraProxy;
//!
//! pub async fn run() -> Result<(), ashpd::Error> {
//!     let connection = zbus::azync::Connection::new_session().await?;
//!     let proxy = CameraProxy::new(&connection).await?;
//!     if proxy.is_camera_present().await? {
//!         proxy.access_camera().await?;
//!
//!         let remote_fd = proxy.open_pipe_wire_remote().await?;
//!         // pass the remote fd to GStreamer for example
//!     }
//!     Ok(())
//! }
//! ```

use super::{HandleToken, DESTINATION, PATH};
use crate::{
    helpers::{call_basic_response_method, call_method},
    Error,
};
use std::{
    collections::HashMap,
    os::unix::prelude::{AsRawFd, RawFd},
};
use zvariant::{Fd, Value};
use zvariant_derive::{DeserializeDict, SerializeDict, TypeDict};

#[derive(SerializeDict, DeserializeDict, TypeDict, Clone, Debug, Default)]
/// Specified options for a [`CameraProxy::access_camera`] request.
struct CameraAccessOptions {
    /// A string that will be used as the last element of the handle.
    handle_token: HandleToken,
}

/// The interface lets sandboxed applications access camera devices, such as web
/// cams.
///
/// Wrapper of the DBus interface: [`org.freedesktop.portal.Camera`](https://flatpak.github.io/xdg-desktop-portal/portal-docs.html#gdbus-org.freedesktop.portal.Camera).
#[derive(Debug)]
#[doc(alias = "org.freedesktop.portal.Camera")]
pub struct CameraProxy<'a>(zbus::azync::Proxy<'a>);

impl<'a> CameraProxy<'a> {
    /// Create a new instance of [`CameraProxy`].
    pub async fn new(connection: &zbus::azync::Connection) -> Result<CameraProxy<'a>, Error> {
        let proxy = zbus::ProxyBuilder::new_bare(connection)
            .interface("org.freedesktop.portal.Camera")
            .path(PATH)?
            .destination(DESTINATION)
            .build_async()
            .await?;
        Ok(Self(proxy))
    }

    /// Get a reference to the underlying Proxy.
    pub fn inner(&self) -> &zbus::azync::Proxy<'_> {
        &self.0
    }

    /// Requests an access to the camera.
    ///
    /// # Specifications
    ///
    /// See also [`AccessCamera`](https://flatpak.github.io/xdg-desktop-portal/portal-docs.html#gdbus-method-org-freedesktop-portal-Camera.AccessCamera).
    #[doc(alias = "AccessCamera")]
    pub async fn access_camera(&self) -> Result<(), Error> {
        let options = CameraAccessOptions::default();
        call_basic_response_method(&self.0, &options.handle_token, "AccessCamera", &(&options))
            .await
    }

    /// Open a file descriptor to the PipeWire remote where the camera nodes are
    /// available.
    ///
    /// # Returns
    ///
    /// File descriptor of an open PipeWire remote.
    ///
    /// # Specifications
    ///
    /// See also [`OpenPipeWireRemote`](https://flatpak.github.io/xdg-desktop-portal/portal-docs.html#gdbus-method-org-freedesktop-portal-Camera.OpenPipeWireRemote).
    #[doc(alias = "OpenPipeWireRemote")]
    pub async fn open_pipe_wire_remote(&self) -> Result<RawFd, Error> {
        // FIXME: figure out what are the possible options
        let options: HashMap<&str, Value<'_>> = HashMap::new();
        let fd: Fd = call_method(&self.0, "OpenPipeWireRemote", &(options)).await?;
        Ok(fd.as_raw_fd())
    }

    /// A boolean stating whether there is any cameras available.
    ///
    /// # Specifications
    ///
    /// See also [`IsCameraPresent`](https://flatpak.github.io/xdg-desktop-portal/portal-docs.html#gdbus-property-org-freedesktop-portal-Camera.IsCameraPresent).
    #[doc(alias = "IsCameraPresent")]
    pub async fn is_camera_present(&self) -> Result<bool, Error> {
        self.0
            .get_property::<bool>("IsCameraPresent")
            .await
            .map_err(From::from)
    }
}
