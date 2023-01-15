import { Auth } from './auth'
import { config } from './config'
import { toastError, toastSuccess } from './toast'

export interface APIResult {
  success: boolean,
  data?: Object | undefined
}

export const getRuntimeConfig = async (): Promise<APIResult> => {
  const url = window.location.href + 'runtime-config.json'
  let resp: Response
  try {
    resp = await fetch(`${url}`)
  } catch (e) {
    return { success: false, data: `${url} has error or unreachable: ${e}` }
  }
  if (!resp.ok) {
    const text = resp.statusText
    return { success: false, data: `${url} returns error (${resp.status}): ${text}` }
  }
  // get Json or text result
  const respClone = resp.clone()
  const js = await resp.json().catch(() => { })
  const txt = await respClone.text().catch(() => { })
  const data = js || txt
  return { success: true, data }
}

export const getApiResult = async (endpoint: string, method: string, payload: Object, successMessage?: string, failureMessagePrefix?: string, logoutCallback?: () => any): Promise<APIResult> => {
  const api = config.getAPIUrl()
  const token = Auth.getToken()
  if (!token) {
    if (failureMessagePrefix) toastError(`${failureMessagePrefix}: unable to read token`)
    if (logoutCallback) logoutCallback()
    return { success: false }
  }

  const headers = {
    'content-type': 'application/json',
    Authorization: `Bearer ${token}`
  }
  const fetchOpts: { [k: string]: any } = { headers, method }
  if (method !== 'get') {
    fetchOpts.body = JSON.stringify(payload)
  }

  let resp: Response
  try {
    const url = `${api}${endpoint}`
    resp = await fetch(`${url}`, fetchOpts)
  } catch (e) {
    let message = e
    if (e instanceof TypeError && e.message === 'Failed to fetch') {
      message = `${api} has error or unreachable`
    }
    if (failureMessagePrefix) toastError(`${failureMessagePrefix}: ${message} `)
    return { success: false }
  }

  if (!resp.ok) {
    // use standard HTTP status text by default
    let text = resp.statusText
    // replace with custom error text if any
    const rt = await resp.text().catch(() => { })
    text = rt || text
    // dont popup expired tokens
    if (failureMessagePrefix && !text.includes('cannot get valid email from token')) {
      toastError(`${failureMessagePrefix} (${resp.status}): ${text} `)
    }
    if (resp.status === 401) {
      Auth.logout()
      if (logoutCallback) logoutCallback()
    }
    return { success: false }
  }

  // get Json or text result
  const respClone = resp.clone()
  const js = await resp.json().catch(() => { })
  const txt = await respClone.text().catch(() => { })
  const data = js || txt
  if (successMessage) toastSuccess(successMessage)
  return { success: true, data }
}
