# Quick start

This uses a pre-existing client app at https://ywinby.web.app/ to quickly demonstrate ywinby on a single computer.

### Run the server
1.  Generate VAPID keys for push notifications:
    ```shell
    docker run --rm mmta/ywinby:latest ./ywinby --generate

    These can be used for push_privkey (-k) and push_pubkey (-p) parameters:
    - privateKey: ******
    - publicKey: *******

    ```
1.  Start `ywinby` with those keys, performing periodic check every 30 seconds, storing data in local JSON files (default):
    ```shell
    docker run --rm mmta/ywinby:latest ./ywinby -k <privateKey> -p <publicKey> -t 30
    ```
> To persist data, mount a directory to `/ywinby/db` inside the container
### Sign in two user accounts
1.  Use Firefox browser to open https://ywinby.web.app/, and sign-in using a Google account (call it the secret `owner`). https://ywinby.web.app/ is pre-configured to use http://localhost:8080 for API server, so it should connect to your local ywinby server.

1.  Use Chrome or Edge browser to open the same URL, and sign-in with a different Google account (call it the `recipient`).

1. On both accounts, click on the 🔕 button to turn-on push notification (it should turn to 🔔). After that, confirm your setup by sending a self-notification with the ✓ button.

### Test sending and recovering a secret message
1.  Create a new message from the owner account to the recipient email address. Set the `Verify your responsiveness every` to 1 minute, and `Max consecutive failure to respond` to 3. Copy the `recipient share` to clipboard before registering the message.

1.  The owner will start receiving reminder in a minute, up to 3 times. This timer will reset every time the owner contacts the API server (e.g. tapping on the notification, logging in to the app, refreshing message list, etc.).

1. If the owner fails to respond in 3 consecutive minutes, the recipient will receive a notification to unlock the secret message. Recipient can then use the `Reveal Content` button and paste-in the `recipient share` earlier to decrypt the message.

And that's it! for a "production" deployment, refer to the guide for [Cloud Run](./cloud-run.md).

> **Note**: use pull-to-refresh on the web client to observe results after each changes. Ywinby supports running in an eco-friendly serverless mode (i.e. not always-on) so there's no automatic client refresh or server-push methods in use.
