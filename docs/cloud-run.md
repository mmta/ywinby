# Ywinby on Cloud Run and Github Action

This setup lets you use ywinby as a free secret inheritance/escrow service by utilizing the free-tiers of Google Cloud Run and Github Actions.

Here ywinby runs in serverless mode, which disables the internal scheduler for periodic checks, and relies on Github action (or other scheduler) to trigger the checks instead. Ywinby "check" means evaluating all messages and sending push notifications as needed to verify secret owners responsiveness, or to inform recipients of any unlocked secrets.

## Setup procedure

1. First sign-up to [Google Cloud Platform](https://console.cloud.google.com/getting-started) and create a new project.

1. Once the project is ready, create an oAuth Client ID from the [API Credential](https://console.cloud.google.com/apis/credentials) menu. Just accept all defaults, and at the end copy and save the generated `Client ID`.

1. Next sign-in to [Firebase console](https://console.firebase.google.com/) using the same Google account. Select the GCP project and create a new Firestore database. Choose `production mode` when prompted.

1. Back to GCP console, make sure you're in the right project then activate and launch [Cloud Shell](https://shell.cloud.google.com/). All of the CLI commands on the next steps should be executed from this cloud shell session. Verify that you're in the correct project with this:
   ```shell
   echo $GOOGLE_CLOUD_PROJECT
   ```  
1. Pull ywinby docker image from docker.io, and push it to your project's container registry.
    ```
    docker pull mmta/ywinby:latest
    docker tag mmta/ywinby:latest gcr.io/$GOOGLE_CLOUD_PROJECT/ywinby:latest
    docker push gcr.io/$GOOGLE_CLOUD_PROJECT/ywinby:latest
    ```
1. Generate VAPID keys for web push notification, then copy and save both the `privateKey` and `publicKey` to an env. variable:
    ```shell
    docker run --rm mmta/ywinby:latest ./ywinby --generate > /tmp/vapid.txt && \
      export PUSH_PRIVKEY=$(cat /tmp/vapid.txt | grep privateKey | awk -F": " '{print $2}') && \
      export PUSH_PUBKEY=$(cat /tmp/vapid.txt | grep publicKey | awk -F": " '{print $2}')
    
    echo $PUSH_PRIVKEY
    echo $PUSH_PUBKEY

    ```
1. Deploy `ywinby` Cloud Run service that uses the previous container image. The result can also be observed from the [GCP console](https://console.cloud.google.com/run).
    ```shell
    gcloud run deploy ywinby \
      --image=gcr.io/$GOOGLE_CLOUD_PROJECT/ywinby:latest \
      --cpu=1 \
      --max-instances=3 \
      --memory=512Mi \
      --min-instances=0 \
      --port=8080 \
      --timeout=60s \
      --no-use-http2 \
      --allow-unauthenticated \
      --set-env-vars="PUSH_PUBKEY"=${PUSH_PUBKEY},"PUSH_PRIVKEY"=${PUSH_PRIVKEY}
    ```
1. The service should be deployed although not yet fully configured. For now, just get the generated URL and save it to an env. variable:

    ```shell
    export BASE_API_PATH=$(gcloud run services list | grep URL | awk -F": " '{print $2}')
    
    echo $BASE_API_PATH
    ```
1. Go to the OAuth Client ID [API Credential](https://console.cloud.google.com/apis/credentials) page from step #2 above, and add that `BASE_API_PATH` URL to `Authorized JavaScript origins`. While on that page, copy the OAuth Client ID and export it as an env variable in the cloud console:
    ```shell
    export CLIENT_ID=***apps.googleusercontent.com

    echo $CLIENT_ID

1. Generate a password for Github action and store it inside an environment variable:
    ```shell
    sudo apt install -y pwgen && \
      export SERVERLESS_TOKEN=$(pwgen -n 20 -1)

    echo $SERVERLESS_TOKEN
    ```

1. Update the Cloud Run service with all the required environment variables to make it work with Firestore and Github. Note this re-uses all of the exported variables from previous steps:

   ```shell

    export VARS=$(echo "
    PROJECT_ID=${GOOGLE_CLOUD_PROJECT},
    STORAGE=firestore,
    PUSH_PRIVKEY=${PUSH_PRIVKEY},
    PUSH_PUBKEY=${PUSH_PUBKEY},
    CLIENT_ID=${CLIENT_ID},
    SERVERLESS_TOKEN=${SERVERLESS_TOKEN},
    BASE_API_PATH=${BASE_API_PATH}
    " | xargs | sed 's/ //g')

    echo $VARS

    gcloud run deploy ywinby --image=gcr.io/$GOOGLE_CLOUD_PROJECT/ywinby:latest --set-env-vars=$VARS
  
    ```
1. The web app should now be accessible from the Cloud Run URL. From here you can test the setup [using multiple accounts](quick-start.md#sign-in-two-user-accounts), substituting https://ywinby.web.app in that section with your own Cloud Run URL. 

1. Next, setup a Github Action (in any public project) to periodically wake ywinby so that it can perform its regular checks. You can use [this workflow](../.github/workflows/cron.yaml) as a starting point. Notice in there how both the target URL and serverless password are stored as a secret.
    
   As an example, to test initiating a scheduled check from the cloud console above (using the previously set env vars):

   ```shell
   curl -sSf -H 'content-type:application/json' \
     ${BASE_API_PATH}/serverless-task \
     -d '{ "token" : "'${SERVERLESS_TOKEN}'"}' 

   task executed successfully
   ```
So now if every thing goes well, you should have a resource-efficient setup that will only be active when there's requests, which should be rare for an app like this. Most of the time the Cloud Run service should scale down to zero instance (nothing is running), and will only be awaken once a day by Github action, or occassionally by users who need to verify themselves or create new messages.

## Limiting access to authorised users

Once you've verified that the setup is working as it should, you can ask would-be users to sign-in using their Google account. This would create an entry in the Firestore `users` collection. After all of them have sign-in at least once, you can prevent others from doing so by adding `BLOCK_REGISTRATION` env. variable:

```shell
gcloud run deploy ywinby \
  --image=gcr.io/$GOOGLE_CLOUD_PROJECT/ywinby:latest \
  --update-env-vars=BLOCK_REGISTRATION=yes
```
And to allow registration again:
```shell
gcloud run deploy ywinby \
  --image=gcr.io/$GOOGLE_CLOUD_PROJECT/ywinby:latest \
  --remove-env-vars=BLOCK_REGISTRATION
```

## Potential problems and solutions

First, check Cloud Run logs for the service to pinpoint the issue.

- If it's related to Firestore access, try creating a dedicated IAM service account for the Cloud run service, and assign the appropriate [Firebase roles](https://firebase.google.com/docs/projects/iam/roles) to it.

- If it's about push notifications, just try repeating the subscription process on the target browser/device, and use the self-notification test and sender/recipient test to confirm.

- If it's about Github Action, just test the curl command first from your own laptop.

