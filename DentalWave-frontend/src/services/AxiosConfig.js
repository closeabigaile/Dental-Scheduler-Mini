import axios from 'axios'
import { getToken, logout } from './AuthService'

const isDesktopShell = import.meta.env.MODE === 'tauri'

/**
 * Axios Request Interceptor
 *
 * This interceptor runs before every axios request.
 *
 * Its purpose is to automatically attach the JWT token
 * to all secured API requests after a user logs in.
 *
 * Without this interceptor, every service method would
 * need to manually add:
 *
 * headers: {
 *     Authorization: getToken()
 * }
 *
 * By configuring it here, all axios requests automatically
 * include the token if one exists.
 */
axios.interceptors.request.use(

    /**
     * Executes before the request is sent.
     *
     * @param config The axios request configuration
     * @returns Updated request configuration
     */
    function (config) {

        // Phase 1 renders the existing UI but must remain isolated from the
        // Spring/H2 reference application and any office data.
        if (isDesktopShell) {
            const error = new Error(
                'The Phase 1 desktop shell does not connect to the scheduling backend.'
            )
            error.code = 'DESKTOP_BACKEND_UNAVAILABLE'
            return Promise.reject(error)
        }

        // Retrieve JWT token from local storage
        const token = getToken()

        // Only attach the Authorization header
        // if a token exists
        if (token) {
            config.headers['Authorization'] = `Bearer ${token}`
        }

        return config
    },

    /**
     * Handles request configuration errors.
     *
     * @param error The request error
     * @returns Rejected promise
     */
    function (error) {
        return Promise.reject(error)
    }
)

// A cached role is not proof that the JWT is still valid. If the backend
// rejects a protected request, clear the stale session and require a fresh
// login instead of leaving the manager on a broken page.
axios.interceptors.response.use(
    (response) => response,
    (error) => {
        const isUnauthorized = error.response?.status === 401
        const isLoginRequest = error.config?.url?.includes('/api/auth/login')

        const suppressAuthRedirect = error.config?.skipAuthRedirect === true

        if (isUnauthorized && !isLoginRequest && !suppressAuthRedirect) {
            logout()
            window.location.replace('/login?session=expired')
        }

        return Promise.reject(error)
    }
)

/**
 * This file does not export anything.
 *
 * Import it once in main.jsx:
 *
 * import './services/AxiosConfig'
 *
 * Doing so registers the interceptor when
 * the application starts.
 */
