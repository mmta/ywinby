
export const config = {
  getAPIUrl: () => {
    return window.localStorage.getItem('api')
  },
  setAPIUrl: (apiUrl: string) => {
    window.localStorage.setItem('api', apiUrl)
  },
  getPushPubkey: () => {
    return window.localStorage.getItem('push_pub_key')
  },
  setPushPubkey: (pushPubKey: string) => {
    window.localStorage.setItem('push_pub_key', pushPubKey)
  }
}
