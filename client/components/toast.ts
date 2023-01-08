import { toast } from 'react-toastify'

export const toastSuccess = (s: string) => {
  try {
    toast.success(s, {
      position: toast.POSITION.BOTTOM_CENTER
    })
  } catch (e) { }
}

export const toastError = (s: string) => {
  try {
    toast.error(s, {
      position: toast.POSITION.BOTTOM_CENTER
    })
  } catch (e) { }
}
