// Provides this.$http with $get / $post / $put / $patch / $delete that match team conventions.
export default defineNuxtPlugin(() => {
  const $get = (url: string, opts: any = {}) => $fetch(url, { method: 'GET', ...opts })
  const $post = (url: string, opts: any = {}) => $fetch(url, { method: 'POST', body: opts.body, ...stripBody(opts) })
  const $put = (url: string, opts: any = {}) => $fetch(url, { method: 'PUT', body: opts.body, ...stripBody(opts) })
  const $patch = (url: string, opts: any = {}) => $fetch(url, { method: 'PATCH', body: opts.body, ...stripBody(opts) })
  const $delete = (url: string, opts: any = {}) => $fetch(url, { method: 'DELETE', ...opts })

  return {
    provide: {
      http: { $get, $post, $put, $patch, $delete },
    },
  }
})

function stripBody(opts: any) {
  const { body, method, ...rest } = opts
  return rest
}
