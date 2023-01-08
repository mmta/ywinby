import { useEffect, useState, FC, useContext } from 'react'
import { getApiResult } from './api'
import { toastError, toastSuccess } from './toast'
import { AppContext, AppContextType } from './appctx'
import { config } from '../components/config'

interface ContainerProps {
}

const WebPush: FC<ContainerProps> = () => {
  const [isSubscribed, setIsSubscribed] = useState(false)
  const [subscription, setSubscription] = useState<PushSubscription>()
  const [registration, setRegistration] = useState<ServiceWorkerRegistration>()
  const { setLoggedIn } = useContext(AppContext) as AppContextType

  const base64ToUint8Array = (base64: string) => {
    const padding = '='.repeat((4 - (base64.length % 4)) % 4)
    const b64 = (base64 + padding).replace(/-/g, '+').replace(/_/g, '/')

    const rawData = window.atob(b64)
    const outputArray = new Uint8Array(rawData.length)

    for (let i = 0; i < rawData.length; ++i) {
      outputArray[i] = rawData.charCodeAt(i)
    }
    return outputArray
  }

  useEffect(() => {
    if (typeof window !== 'undefined' && 'serviceWorker' in navigator && 'workbox' in window) {
      // run only in browser
      navigator.serviceWorker.ready.then(reg => {
        reg.pushManager.getSubscription().then(sub => {
          if (sub && !(sub.expirationTime && Date.now() > sub.expirationTime - 5 * 60 * 1000)) {
            setSubscription(sub)
            setIsSubscribed(true)
          }
        })
        setRegistration(reg)
      })
    }
  }, [])

  const subscribeButtonOnClick = async (event: any) => {
    event.preventDefault()
    const pubKey = config.getPushPubkey()
    if (!registration || !pubKey) return
    const sub = await registration.pushManager.subscribe({
      userVisibleOnly: true,
      applicationServerKey: base64ToUint8Array(pubKey)
    })
    if (!sub) {
      toastError('browser failed to subscribe for push notification')
      return
    }
    const result = await getApiResult('/subscribe-user', 'post', { subscription: sub }, 'push notification enabled successfully', 'failed to enable push notification', () => setLoggedIn(false))
    if (result.success) {
      setSubscription(sub)
      setIsSubscribed(true)
    }
  }

  const unsubscribeButtonOnClick = async (event: any) => {
    event.preventDefault()
    if (!subscription) return
    const unsub = await subscription.unsubscribe()
    if (!unsub) {
      toastError('browser failed to unsubscribe from push notification')
      return
    }
    setSubscription(undefined)
    setIsSubscribed(false)
    toastSuccess('push notification disabled successfully')
    // API result doesn't matter, user will no longer receive notifications after the above
    await getApiResult('/unsubscribe-user', 'post', {})
  }

  const testNotification = async () => {
    await getApiResult('/test-notification', 'post', {}, 'request sent! check the server logs if you dont receive a message soon', 'fail to request for test notification', () => setLoggedIn(false))
  }

  return (
    <>
      {
        registration
          ? <div>
          <button className={isSubscribed ? 'button is-light is-outlined is-warning' : 'button is-outlined is-danger'}
            onClick={isSubscribed ? unsubscribeButtonOnClick : subscribeButtonOnClick}>
            {isSubscribed ? '🔔' : '🔕'}
          </button>
          <span></span>
          { isSubscribed
            ? <>
          <button className='button is-light is-outlined is-warning' onClick={testNotification}><span className='gg-check-o'></span>
          </button>
          <span></span>
          </>
            : <></>
          }
        </div>
          : <></>
      }
    </>
  )
}

export default WebPush
