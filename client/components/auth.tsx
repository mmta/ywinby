import { googleLogout } from '@react-oauth/google'
const tokenName = 'jwt_token'

export const Auth = {
  isLoggedIn: () => {
    const token = window.localStorage.getItem(tokenName)
    return !!token
  },
  getToken: () => {
    return window.localStorage.getItem(tokenName)
  },
  setToken: (token: string) => {
    window.localStorage.setItem(tokenName, token)
  },
  setEmail: (email: string) => {
    window.localStorage.setItem('email', email)
  },
  getEmail: () => {
    return window.localStorage.getItem('email') || ''
  },
  logout: () => {
    window.localStorage.removeItem(tokenName)
    googleLogout()
  }
}
