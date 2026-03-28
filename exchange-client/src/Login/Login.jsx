import { useState } from 'react'

function Login({user, setUser}) {
    if (user) {return <div>Loading...</div>}
    const [name, setName] = useState("")
    const [password, setPassword] = useState("")
    return (
        <div className="login-page">
        <div className="login-title">Columbia Trading Competition</div>
        <div className="login-card">
        <form onSubmit={(e) => {e.preventDefault(); setUser({uid: name, pwd: password})}}
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
