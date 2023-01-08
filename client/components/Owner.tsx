import { useCallback, useContext, useEffect, useState } from 'react'
import { AppContext, AppContextType } from './appctx'
import { getApiResult } from './api'

export default function Owner () {
  const { setLoggedIn, setShowOwner } = useContext(AppContext) as AppContextType

  const [ready, setReady] = useState(false)
  const [result, setResult] = useState(false)

  const ownerPong = useCallback(
    async (): Promise<boolean> => {
      const result = await getApiResult('/user-pong', 'post', {})
      return result.success
    }, [])

  useEffect(() => {
    const f = async () => {
      const r = await ownerPong()
      setResult(r)
      setReady(true)
    }
    if (!result) f()
  }, [ownerPong, ready, result, setReady])

  const handleClick = () => {
    if (!result) setLoggedIn(false)
    setShowOwner(false)
  }

  return (
    <> {!ready
      ? ''
      : <div className={'modal is-active'}>
        <div className="modal-background"></div>
        <div className="modal-card">
          <header className="modal-card-head">
            <p className="modal-card-title">Owner verification</p>
            <button className="delete" aria-label="close" onClick={handleClick}></button>
          </header>
          <section className="modal-card-body">
            <div className="mb-5">
              { result
                ? <p>verification accepted, thanks🙏</p>
                : <p>error occurred, please relogin to auto-verify yourself 🙏</p>
              }
            </div>
          </section>
        </div>
      </div>
    }
    </>
  )
}
