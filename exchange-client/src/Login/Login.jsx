import { useState } from 'react'

function Login({user, setUser}) {
    if (user) {return <div>Loading...</div>}
    const saved = JSON.parse(localStorage.getItem('credentials') || '{}')
    const [name, setName] = useState(saved.uid || "")
    const [password, setPassword] = useState(saved.pwd || "")

    function handleSubmit(e) {
        e.preventDefault()
        const creds = {uid: name, pwd: password}
        localStorage.setItem('credentials', JSON.stringify(creds))
        setUser(creds)
    }

    return (
        <div className="login-page">
        <div className="login-title">Columbia Trading Competition</div>
        <div className="login-card">
        <form onSubmit={handleSubmit}
            style={{display:"flex", flexDirection:"column", gap:"12px"}}
        >
        <input value={name} type="text" onChange={(e) => setName(e.target.value)} placeholder='Username' required/>
        <input value={password} type="password" onChange={(e) => setPassword(e.target.value)} placeholder='Password' required/>
        <input type="submit" value="Join"/>
        </form>
        </div>
        </div>
    )
}

export default Login
