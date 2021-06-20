use crate::{
    helpers::{call_method, property},
    Error, HandleToken,
};
use futures::prelude::stream::*;
use serde::{Serialize, Serializer};
use std::{collections::HashMap, convert::TryFrom};
use zvariant::{ObjectPath, OwnedValue, Signature};

pub type SessionDetails = HashMap<String, OwnedValue>;

/// The Session interface is shared by all portal interfaces that involve long
/// lived sessions. When a method that creates a session is called, if
/// successful, the reply will include a session handle (i.e. object path) for a
/// Session object, which will stay alive for the duration of the session.
///
/// The duration of the session is defined by the interface that creates it.
/// For convenience, the interface contains a method Close(), and a signal
/// `org.freedesktop.portal.Session::Closed`. Whether it is allowed to directly
/// call Close() depends on the interface.
///
/// The handle of a session will be of the form
/// `/org/freedesktop/portal/desktop/session/SENDER/TOKEN`, where SENDER is the
/// callers unique name, with the initial ':' removed and all '.' replaced by
/// '_', and TOKEN is a unique token that the caller provided with the
/// session_handle_token key in the options vardict of the method creating the
/// session.
///
/// The token that the caller provides should be unique and not guessable.
/// To avoid clashes with calls made from unrelated libraries, it is a good idea
/// to use a per-library prefix combined with a random number.
///
/// A client who started a session vanishing from the D-Bus is equivalent to
/// closing all active sessions made by said client.
#[derive(Debug)]
pub struct SessionProxy<'a>(zbus::azync::Proxy<'a>, zbus::azync::Connection);

impl<'a> SessionProxy<'a> {
    /// Create a new instance of [`SessionProxy`].
    ///
    /// **Note** A [`SessionProxy`] is not supposed to be created manually.
    pub async fn new(
        connection: &zbus::azync::Connection,
        path: ObjectPath<'a>,
    ) -> Result<SessionProxy<'a>, Error> {
        let proxy = zbus::ProxyBuilder::new_bare(connection)
            .interface("org.freedesktop.portal.Session")
            .path(path)?
            .destination("org.freedesktop.portal.Desktop")
            .build_async()
            .await?;
        Ok(Self(proxy, connection.clone()))
    }

    pub(crate) async fn from_unique_name(
        connection: &zbus::azync::Connection,
        handle_token: &HandleToken,
    ) -> Result<SessionProxy<'a>, crate::Error> {
        let unique_name = connection.unique_name().unwrap();
        let unique_identifier = unique_name.trim_start_matches(':').replace('.', "_");
        let path = zvariant::ObjectPath::try_from(format!(
            "/org/freedesktop/portal/desktop/session/{}/{}",
            unique_identifier, handle_token
        ))?;
        SessionProxy::new(connection, path).await
    }

    /// Get a reference to the underlying Proxy.
    pub fn inner(&self) -> &zbus::azync::Proxy<'_> {
        &self.0
    }

    /// Emitted when a session is closed.
    pub async fn receive_closed(&self) -> Result<SessionDetails, Error> {
        let mut stream = self.0.receive_signal("Closed").await?;
        let message = stream.next().await.ok_or(Error::NoResponse)?;
        message.body::<SessionDetails>().map_err(From::from)
    }

    /// Closes the portal session to which this object refers and ends all
    /// related user interaction (dialogs, etc).
    pub async fn close(&self) -> Result<(), Error> {
        call_method(&self.0, "Close", &()).await
    }

    /// The version of this DBus interface.
    pub async fn version(&self) -> Result<u32, Error> {
        property(&self.0, "version").await
    }
}

impl<'a> Serialize for SessionProxy<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        zvariant::ObjectPath::serialize(self.0.path(), serializer)
    }
}

impl<'a> zvariant::Type for SessionProxy<'a> {
    fn signature() -> Signature<'static> {
        zvariant::ObjectPath::signature()
    }
}
