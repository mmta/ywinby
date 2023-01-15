import { useContext, useState, FC } from 'react'
import { secret } from '../external/secrets'
import { toastError, toastSuccess } from './toast'
import TextareaAutosize from 'react-textarea-autosize'
import { AppContext, AppContextType } from './appctx'
import { getApiResult } from './api'

const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms))

interface ContainerProps { }

const NewMessage: FC<ContainerProps> = () => {
  const { toggleRefresh, setShowNewMessage, setLoggedIn } = useContext(AppContext) as AppContextType

  const [secretMessage, setSecretMessage] = useState('')
  const [ownerShare, setOwnerShare] = useState('')
  const [recipientShare, setRecipientShare] = useState('')
  const [systemShare, setSystemShare] = useState('')
  const [verifyTime, setVerifyTime] = useState(1)
  const [verifyLimit, setVerifyLimit] = useState(1)
  const [email, setEmail] = useState('')

  const [thresholdTime, setThresholdTime] = useState('1')
  const [thresholdTimeUnit, setThresholdTimeUnit] = useState('day(s)')

  const [modalActive, setModalActive] = useState('') // is-active to activate

  const splitShare = (ev: any) => {
    setSecretMessage(ev.target.value)
    const hex = secret.str2hex(ev.target.value)
    const shares = secret.share(hex, 3, 2)
    setOwnerShare(shares[0])
    setRecipientShare(shares[1])
    setSystemShare(shares[2])
  }

  const updateVerifyTime = (ev: any) => {
    if (ev.target.value > 99 || ev.target.value < 0) return
    setVerifyTime(ev.target.value)
    const maxTime = Number(ev.target.value) * verifyLimit
    setThresholdTime(`${maxTime}`)
  }

  const updateVerifyLimit = (ev: any) => {
    if (ev.target.value > 9 || ev.target.value < 0) return
    setVerifyLimit(ev.target.value)
    const maxTime = verifyTime * Number(ev.target.value)
    setThresholdTime(`${maxTime}`)
  }

  const copyToClipboard = (shareOwner: string) => {
    let target = ownerShare
    switch (shareOwner) {
      case 'recipient':
        target = recipientShare
        break
      case 'system':
        target = systemShare
        break
    }
    if (target === 'undefined') {
      toastError('write the secret message first')
    } else {
      if (window.navigator) {
        navigator.clipboard.writeText(String(target))
      }
      toastSuccess(`${shareOwner} share has been copied to clipboard`)
    }
  }

  const registerMessageHandler = async () => {
    setModalActive('')
    const recipient = (document.getElementById('email') as HTMLInputElement).value
    let verifyEveryMinutes = parseInt((document.getElementById('verify-time') as HTMLInputElement).value)
    const maxFailedVerification = parseInt((document.getElementById('verify-limit') as HTMLInputElement).value)

    switch (thresholdTimeUnit.trim()) {
      case 'day(s)': {
        verifyEveryMinutes = verifyEveryMinutes * 60 * 24
        break
      }
      case 'month(s)': {
        verifyEveryMinutes = verifyEveryMinutes * 60 * 24 * 30
        break
      }
    }
    const payload = {
      message: {
        verify_every_minutes: verifyEveryMinutes,
        max_failed_verification: maxFailedVerification,
        recipient,
        system_share: systemShare
      }
    }
    const result = await getApiResult('/message', 'post', payload, 'Message registered successfully', 'error registering message', () => setLoggedIn(false))
    if (result.success) {
      toggleRefresh()
    }
    await sleep(10)
    setShowNewMessage(false)
  }

  return (
    <>
      <div className={`modal ${modalActive}`}>
        <div className="modal-background"></div>
        <div className="modal-card">
          <header className="modal-card-head">
            <p className="modal-card-title">Confirm registration</p>
            <button className="delete" aria-label="close" onClick={() => setModalActive('')}></button>
          </header>
          <section className="modal-card-body">
            This will upload the system&rsquo;s share to the server, and start verifying your responsiveness according to the schedule. Continue?
          </section>
          <footer className="modal-card-foot">
            <button className="button is-success" onClick={() => registerMessageHandler()}>Yes</button>
            <button className="button is-danger is-light is-outlined" onClick={() => setModalActive('')}>No</button>
          </footer>
        </div>
      </div>
      <form className="box has-background-info-light" onSubmit={e => e.preventDefault()}>
        <div>
          <header className="subtitle">Recipient and Message</header>
          <div>
            <label >Recipient google ID (email)</label>
            <input className='input' type='email' id='email' placeholder='enter the recipient email' value={email} onChange={ev => setEmail(ev.target.value)} required />
          </div>
          <div>
            <label >Secret message</label>
            <TextareaAutosize required className="textarea" id='secret-message' placeholder='your secret message' auto-grow='true' value={secretMessage} onChange={(ev) => splitShare(ev)} />
          </div>
          <div className="mt-1 level">
            <button type='button' className="button m-1 is-link is-outlined is-fullwidth" disabled={secretMessage === ''} onClick={() => copyToClipboard('your')}>Copy your share</button>
            <button type='button' className="button m-1 is-link is-outlined is-fullwidth" disabled={secretMessage === ''} onClick={() => copyToClipboard('recipient')}>Copy recipient&rsquo;s share</button>
            <button type='button' className="button m-1 is-link is-outlined is-fullwidth" disabled={secretMessage === ''} onClick={() => copyToClipboard('system')}>Copy system&rsquo;s share</button>
          </div>
          <div>
            <p className="mt-2">Two shares will be enough to recover your secret message. Verify with a third-party software <a target='_blank' href='https://iancoleman.io/shamir/' rel="noreferrer">here</a>.</p>
          </div>
          <header className="subtitle mt-5">Schedule and Limit</header>
          <div className="level">
            <div className="level-left">
              <div className="level-item">
                <label>Verify your responsiveness every (1-99)</label>
              </div>
            </div>
            <div className="level-right">
              <div className="level-item">
                <input className='level-item input' id='verify-time' type="number" placeholder="00" min="1" max="99" value={verifyTime} onChange={(ev) => updateVerifyTime(ev)}></input>
              </div>
              <div className="level-item">
                <div className='select'>
                  <select id='verify-unit' defaultValue={thresholdTimeUnit} onChange={(ev) => setThresholdTimeUnit(ev.target.value)}>
                    <option value="minute(s)">Minute(s)</option>
                    <option value="day(s)">Day(s)</option>
                    <option value="month(s)">Month(s)</option>
                  </select>
                </div>
              </div>
            </div>
          </div>
          <div className="level">
            <div className="level-left">
              <label>Max consecutive failure to respond (1-9)</label>
            </div>
            <div className="level-right">
              <input className='level-item input' id='verify-limit' type="number" placeholder="0" min="1" max="9" value={verifyLimit} onChange={(ev) => updateVerifyLimit(ev)}></input>
            </div>
          </div>
          <div className="mt-4">
            <label>System&rsquo;s secret share will be sent to recipient if you&rsquo;re not responsive after {thresholdTime} consecutive {thresholdTimeUnit}</label>
          </div>
          <div>
            <button type='submit' disabled={verifyLimit < 1 || verifyLimit > 9 || verifyTime < 1 || verifyTime > 99 || email === '' || secretMessage === '' } className='mt-5 button is-outlined is-link is-fullwidth' onClick={(ev) => { ev.preventDefault(); setModalActive('is-active') }} >Register this message</button>
          </div>
        </div>
      </form>
      <br></br>
    </>
  )
}

export default NewMessage
