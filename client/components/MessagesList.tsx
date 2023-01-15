import { useCallback, useContext, useEffect, useState, FC } from 'react'
import { AppContext, AppContextType } from './appctx'
import { IMessage } from './messages'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import duration from 'dayjs/plugin/duration'
import DecryptMessageContainer from './DecryptMessage'
import { getApiResult } from './api'
import PullToRefresh from 'react-simple-pull-to-refresh'
import { toastError } from './toast'
import { confirmAlert } from 'react-confirm-alert'
import LoadingOverlay from 'react-loading-overlay-ts'

interface ContainerProps { }

const MessageList: FC<ContainerProps> = () => {
  const [showTable, setShowTable] = useState(false)
  const { messages, setMessages, shouldRefresh, toggleRefresh, setLoggedIn, myEmail, loading, setLoading } = useContext(AppContext) as AppContextType

  const [verifyRecipient, setVerifyRecipient] = useState('')
  const [verifySystemShare, setVerifySystemShare] = useState('')
  const [verifyCounterPart, setVerifyCounterPart] = useState('')
  const [showVerification, setShowVerification] = useState(false)

  dayjs.extend(duration)
  dayjs.extend(relativeTime)

  const listMessages = useCallback(
    async (promptAfter: boolean): Promise<any> => {
      let messages: IMessage[] = []
      const successMessage = promptAfter ? 'message list refreshed successfully' : undefined
      setLoading(true)
      const { success, data } = await getApiResult('/message-list', 'get', {}, successMessage, 'failed to get messages', () => setLoggedIn(false))
      if (success && data) {
        messages = data as unknown as IMessage[]
      }
      if (messages.length > 0) {
        setMessages(messages)
        setShowTable(true)
      } else {
        setShowTable(false)
      }
      setLoading(false)
    }, [setLoading, setLoggedIn, setMessages])

  useEffect(() => {
    const f = async () => await listMessages(false)
    f()
  }, [listMessages, setLoading, shouldRefresh])

  const deleteMessage = async (id: string): Promise<any> => {
    const payload = { message_id: id }
    const result = await getApiResult('/message', 'delete', payload, 'Message deleted successfully', 'error deleting message', () => setLoggedIn(false))
    if (result.success) toggleRefresh()
  }

  const displayDecryption = (counterPart: string, recipient: string, systemShare: string) => {
    setVerifyCounterPart(counterPart)
    setVerifyRecipient(recipient)
    setVerifySystemShare(systemShare)
    setShowVerification(true)
  }

  const confirmDeleteMessage = (id: string) => {
    confirmAlert({
      customUI: ({ onClose }) => {
        return (
          <div className={'modal is-active'}>
            <div className="modal-background"></div>
            <div className="modal-card">
              <header className="modal-card-head">
                <p className="modal-card-title">Are you sure?</p>
                <button className="delete" aria-label="close" onClick={onClose}></button>
              </header>
              <section className="modal-card-body">
                This will <strong>permanently</strong> delete this message, including its system&rsquo;s secret share.
              </section>
              <footer className="modal-card-foot">
                <button className="button is-success" onClick={onClose}>No</button>
                <button className="button is-danger is-light"
                  onClick={() => {
                    deleteMessage(id)
                    onClose()
                  }}
                >Yes</button>
              </footer>
            </div>
          </div>
        )
      }
    })
  }

  const sendPingNotification = async (target: string) => {
    await getApiResult('/test-notification', 'post', { recipient: target }, 'request sent!', `fail to send notification request for ${target}`, () => setLoggedIn(false))
  }

  const confirmPingNotification = (recipient: string) => {
    confirmAlert({
      customUI: ({ onClose }) => {
        return (
          <div className={'modal is-active'}>
            <div className="modal-background"></div>
            <div className="modal-card">
              <header className="modal-card-head">
                <p className="modal-card-title">Ping {recipient}</p>
                <button className="delete" aria-label="close" onClick={onClose}></button>
              </header>
              <section className="modal-card-body">
                This will send a message asking {recipient} to log in to Ywinby
              </section>
              <footer className="modal-card-foot">
              <button className="button is-success"
                  onClick={() => {
                    sendPingNotification(recipient)
                    onClose()
                  }}
                >Yes</button>
                <button className="button" onClick={onClose}>No</button>
              </footer>
            </div>
          </div>
        )
      }
    })
  }

  return (
    <>{
      showVerification
        ? <DecryptMessageContainer counterPart={verifyCounterPart} recipient={verifyRecipient} me={myEmail} systemShare={verifySystemShare} closeCallback={function () { setShowVerification(false) }} />
        : ''
      }
      <LoadingOverlay active={loading} spinner fadeSpeed={1000} text='refreshing ...' >
      {
        !showTable
          ? <PullToRefresh onRefresh={() => listMessages(true)} pullingContent='' refreshingContent=''>{
            !loading
              ? <div className='m-3'>
              <p className='mb-3'>Welcome to Ywinby!</p>
              <p className='mb-3'>First, make sure you and your counterpart have both subscribed ☝ for notification.</p>
              <p className='mb-3'>After that you can use the <span className='has-text-primary'>New</span> button to start!</p>
              <p className='mb-3'>Anytime you want to refresh, just pull down and release around this area.</p>
              <br></br><br></br><br></br><br></br>
            </div>
              : <div className='container m-6'><br></br><br></br><br></br><br></br><br></br></div>
          }
          </PullToRefresh>
          : <>
            <PullToRefresh onRefresh={() => listMessages(true)} pullingContent='' refreshingContent=''>
              <>

                {
                  messages.length > 0 &&
                  messages.filter((k) => k.owner === myEmail).map((k) =>
                    <div key={k.id} className="card m-3">
                      <header className="card-header">
                        <p className="card-header-title">
                          <button className='button is-white has-text-weight-semibold'
                            onClick={() => confirmPingNotification(k.recipient)}>{k.recipient}</button>
                            <span className="tag is-rounded">{dayjs.unix(k.created_ts).format('DD MMM YYYY')}</span>
                        </p>
                      </header>
                      <div className="card-content">
                        <div className="content">
                          <p><span className="tag is-warning is-light is-rounded">outbound</span>
                          {k.revealed ? <span className="tag is-link is-light is-rounded">revealed</span> : <></>}</p>
                          {k.revealed
                            ? <>This message has been revealed to the recipient!</>
                            : <>
                            The system&rsquo;s secret share will be revealed to the recipient above if you don&rsquo;t open this app for {dayjs.duration(k.verify_every_minutes * k.max_failed_verification, 'minute').humanize()} straight.
                            A reminder will be sent every {dayjs.duration(k.verify_every_minutes, 'minutes').humanize().replace('a ', '').replace('an ', '')}, and will reset every time you login.
                            <br></br><br></br>
                            The recipient was last seen {dayjs.unix(k.recipient_last_seen).fromNow()}. Tap on their email to send a ping notification message.
                          </>
                          }
                          <br />
                        </div>
                      </div>
                      <footer className="card-footer">
                        <a onClick={() => { displayDecryption(k.recipient, k.recipient, k.system_share) }} className="card-footer-item has-text-primary-dark">Verify Content</a>
                        <a onClick={() => confirmDeleteMessage(k.id)} className="card-footer-item has-text-danger-dark">Delete Message</a>
                      </footer>
                    </div>
                  )}
                {
                  messages.filter((k) => k.recipient === myEmail).map((k) =>
                    <div key={k.id} className="card m-3">
                      <header className="card-header">
                        <p className="card-header-title">
                        <button className='button is-white has-text-weight-semibold'
                            onClick={() => confirmPingNotification(k.owner)}>{k.owner}</button>
                            <span className="tag is-rounded">{dayjs.unix(k.created_ts).format('DD MMM YYYY')}</span>
                        </p>
                      </header>
                      <div className="card-content">
                        <div className="content">
                          <p>
                          <span className="tag is-warning is-light is-rounded">inbound</span>
                          {k.system_share ? <span className="tag is-primary is-light is-rounded">unlocked</span> : <></>}</p>
                          {k.system_share
                            ? <>You can reveal this message now!</>
                            : <>You can reveal this message {dayjs.duration(k.verify_every_minutes * k.max_failed_verification, 'minute').humanize()} after
                              the owner above stop logging in to the app. Owner was last seen {dayjs.unix(k.owner_last_seen).fromNow()},
                              so this unlocks {dayjs.unix(k.owner_last_seen).add((k.verify_every_minutes * k.max_failed_verification), 'minute').fromNow()}. You will receive an alert once
                              that happens.

                            </>
                          }
                          <br />
                        </div>
                      </div>
                      <footer className="card-footer">
                        <a onClick={() => { k.system_share ? displayDecryption(k.owner, k.recipient, k.system_share) : toastError('you can\'t reveal this message yet!') }} className="card-footer-item has-text-primary-dark">Reveal Content</a>
                        <a onClick={() => { k.system_share ? confirmDeleteMessage(k.id) : toastError('you can\'t delete this message yet!') }} className="card-footer-item has-text-danger-dark">Delete Message</a>
                      </footer>
                    </div>
                  )}
                <br></br>
                <br></br>
                <br></br>
              </>
              </PullToRefresh>
          </>
      }
      </LoadingOverlay>
    </>
  )
}

export default MessageList
