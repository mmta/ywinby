import { FC, useEffect, useState } from 'react'
import { secret } from '../external/secrets'
import TextareaAutosize from 'react-textarea-autosize'

interface ContainerProps {
  counterPart: string,
  systemShare: string,
  me: string,
  recipient: string,
  closeCallback: () => void
}

const DecryptMessage: FC<ContainerProps> = ({ counterPart, recipient, me, systemShare, closeCallback }: ContainerProps) => {
  const [myShare, setMyShare] = useState('')
  const [secretMessage, setSecretMessage] = useState('')
  const [errMessage, setErrMessage] = useState('')

  // eslint-disable-next-line no-unused-vars
  enum UserType {
    // eslint-disable-next-line no-unused-vars
    OWNER,
    // eslint-disable-next-line no-unused-vars
    RECIPIENT
  }

  const myType:UserType = me === recipient ? UserType.RECIPIENT : UserType.OWNER

  useEffect(() => {
    if (myShare === '' || systemShare === '') return
    try {
      const hex = secret.combine([myShare, systemShare])
      const msg = secret.hex2str(hex)
      setSecretMessage(msg)
      setErrMessage('')
    } catch (e) {
      setSecretMessage('')
      setErrMessage(`${e}`)
    }
  }, [myShare, systemShare])

  return (
    <>
      <div className={'modal is-active'}>
        <div className="modal-background"></div>
        <div className="modal-card">
          <header className="modal-card-head">
            <p className="modal-card-title">{ myType === UserType.OWNER ? 'Verify' : 'Reveal'} secret message</p>
            <button className="delete" aria-label="close" onClick={() => closeCallback()}></button>
          </header>
          <section className="modal-card-body">
            <div className="mb-5">
              <label>{ myType === UserType.OWNER ? 'Recipient' : 'Sender' } google ID (email): {`${counterPart}`}</label>
            </div>
            <div className="mb-5">
              <label>Paste-in your secret share here:</label>
              <textarea className="textarea" id='owner-share' placeholder='your secret share' onChange={(ev) => setMyShare(ev.target.value)} />
              { errMessage === '' ? '' : <code>{errMessage}</code> }
            </div>
            <div className="mb-4">
              <label>Decrypted secret message will be shown below:</label>
              <TextareaAutosize className="textarea" readOnly id='secret-message' placeholder='your secret message' auto-grow='true' value={secretMessage} />
            </div>
          </section>
          <footer className="modal-card-foot">
          { myType === UserType.OWNER
            ? 'Make sure you get the right message back. Later on the recipient will be able to do the same with their secret share.'
            : 'Copy the decrypted message for later use. You can then delete this message from the list to prevent further push notification on it.'
          }
          </footer>
        </div>
      </div>
    </>
  )
}

export default DecryptMessage
