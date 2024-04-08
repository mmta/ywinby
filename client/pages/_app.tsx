import '../styles/styles.scss'
import 'react-toastify/dist/ReactToastify.css'
import { ToastContainer } from 'react-toastify'

import { IMessage } from '../components/messages'
import { AppContext } from '../components/appctx'
import { config } from '../components/config'
import { getRuntimeConfig } from '../components/api'

import type { AppProps } from 'next/app'
import { useEffect, useState } from 'react'
import Head from 'next/head'

import '@fontsource/roboto/400.css'
import { GoogleOAuthProvider } from '@react-oauth/google'

export default function App ({ Component, pageProps }: AppProps) {
  const [messages, setMessages] = useState<IMessage[]>([])
  const [showNewMessage, setShowNewMessage] = useState(false)
  const [shouldRefresh, setShouldRefresh] = useState(false)
  const [myEmail, setMyEmail] = useState('')
  const [showOwner, setShowOwner] = useState(false)
  const [showLoginPrompt, setShowLoginPrompt] = useState(false)
  const [loggedIn, setLoggedIn] = useState(false)
  const [configLoaded, setConfigLoaded] = useState(false)
  const [loading, setLoading] = useState(false)

  function toggleRefresh () {
    setShouldRefresh(val => !val)
  }

  useEffect(() => {
    const f = async () => {
      // first we get the runtime config from where the UI is hosted, this is just
      // to get the API server URL
      const { success, data } = await getRuntimeConfig(window.location.href)
      const apiUrl = (data as any).api_url || ''
      if (success) {
        config.setAPIUrl(apiUrl)
        // then we get the runtime config from the API server
        const { success, data } = await getRuntimeConfig(apiUrl)
        if (success) {
          config.setPushPubkey((data as any).push_pubkey_base64)
          setConfigLoaded(true)
        } else {
          alert('cannot connect to API server ' + apiUrl + '. Please make sure the server is running and reachable')
        }
      } else {
        alert(data)
      }
    }
    f()
  }, [])

  return (
    !configLoaded
      ? <></>
      : <>
    <Head><title>Ywinby</title></Head>
    <GoogleOAuthProvider clientId="806452214643-l366imhlc0c64coebiik6t3otfjatis3.apps.googleusercontent.com"
        onScriptLoadError={async () => {
          setShowLoginPrompt(false)
        }}
        onScriptLoadSuccess={async () => {
          setShowLoginPrompt(true)
        }}
        >
    <AppContext.Provider
      value={{
        messages,
        setMessages,
        myEmail,
        setMyEmail,
        loggedIn,
        setLoggedIn,
        showNewMessage,
        setShowNewMessage,
        shouldRefresh,
        toggleRefresh,
        showOwner,
        setShowOwner,
        showLoginPrompt,
        setShowLoginPrompt,
        loading,
        setLoading
      }}>
      <ToastContainer />
      <Component {...pageProps} />
    </AppContext.Provider>
    </GoogleOAuthProvider>
    </>
  )
}
