'use strict'

self.addEventListener('waiting', () => {
  window.workbox.messageSkipWaiting()
})

self.addEventListener('controlling', () => {
  window.location.reload()
})

self.addEventListener('push', function (event) {
  const data = JSON.parse(event.data.text())
  event.waitUntil(
    registration.showNotification(data.title, {
      body: data.message,
      tag: data.tag,
      icon: '/icons/icon-192x192.png'
    })
  )
})

self.addEventListener('notificationclick', function (event) {
  event.notification.close()
  event.waitUntil(
    clients.matchAll({ type: 'window', includeUncontrolled: true }).then((clientList) => {
      if (clientList.length > 0) {
        let client = clientList[0]
        for (let i = 0; i < clientList.length; i++) {
          if (clientList[i].focused) {
            client = clientList[i]
          }
        }
        return client.focus().then(client=> client.postMessage({tag: event.notification.tag}))
      }
      clients.openWindow('/')
        .then((client) => {
          setTimeout(() => {
            client.postMessage({
              tag: event.notification.tag
            })
          },3000)
        })
        .catch(e => {
          console.log('cannot open window (happens in Edge installed PWA): ', e)
        })
      })
    )
})
