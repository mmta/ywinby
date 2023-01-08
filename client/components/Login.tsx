import { Auth } from './auth'
import { GoogleLogin } from '@react-oauth/google'
import { AppContext, AppContextType } from './appctx'
import { FC, useContext, useState, useEffect } from 'react'
import { toastError } from './toast'
import { ClipLoader } from 'react-spinners'

const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms))

const Login: FC<{}> = () => {
  const { setLoggedIn, showLoginPrompt, setLoading } = useContext(AppContext) as AppContextType
  const [loginLoading, setLoginLoading] = useState(true)

  const decodeEmail = (credential: string) => {
    const payload = credential.split('.')[1]
    const buf = Buffer.from(payload, 'base64')
    const json = JSON.parse(buf.toString())
    return json.email
  }

  useEffect(() => {
    const f = async () => {
      await sleep(1000)
      setLoginLoading(false)
    }
    f()
  }, [loginLoading, showLoginPrompt])

  return (
    <>
      <div className="level">
      <div className="level-item"><ClipLoader loading={loginLoading}/></div>
        </div>
        { !loginLoading
          ? <>
          { showLoginPrompt
            ? <div>
            <div className="level">
              <div className="level-item">
              <GoogleLogin
                size='large'
                onSuccess={resp => {
                  if (resp?.credential) {
                    try {
                      Auth.setToken(resp.credential ? resp.credential : '')
                      Auth.setEmail(decodeEmail(resp.credential))
                      setLoading(true)
                      setLoggedIn(true)
                    } catch (e) {
                      toastError(`error during login process: ${e}`)
                    }
                  }
                }}
                onError={() => {
                  toastError('cannot complete signin with Google')
                }}
              />
              </div>
            </div>
            <div className="ml-6 mr-6 level">
              <div className="level-item">Signin 👆 to use the app, or read the documentation first 👇</div>
            </div>
            </div>
            : <div className="ml-6 mr-6 level">
              <div className="level-item">Cannot load Google login script, please check your network connection and restart the app 🙏</div>
            </div>
          }
          </>
          : <></>
        }
    <p></p>
    </>
  )
}

export default Login
