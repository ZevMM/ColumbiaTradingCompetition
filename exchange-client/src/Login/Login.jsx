import { useState, useEffect } from 'react'



function Login({user, setUser}) {
    if (user) {return <div>Loading...</div>}
    const [name, setName] = useState("")
    const [password, setPassword] = useState("")
    return (
        <div style={{width:"100%", height:"100%", display:"flex",
            flexDirection:"column", alignItems:"center", justifyContent:"center", position:"absolute"
        }}>
        <div style={{fontFamily:"IBM Plex Sans", color: "white", fontSize:"40px", marginBottom:"10px"}}>Welcome to the Columbia Trading Competition</div>
        
        <form onSubmit={(e) => {e.preventDefault; setUser({uid: name, pwd: password})}}
            style={{width:"100%", display:"flex", flexDirection:"column", alignItems:"center"}}
        >
        <input style ={{marginTop:"10px", width:"15%", fontFamily:"IBM Plex Sans Condensed"}} value={name} onChange={(e) => setName(e.target.value)} placeholder='Username' required/>
        <input style ={{margin:"5px", width:"15%", fontFamily:"IBM Plex Sans Condensed"}} value={password} onChange={(e) => setPassword(e.target.value)} placeholder='Password' required/>
        <input style ={{margin:"5px", width: "15%", background:"rgb(10, 10, 18)", border:"1px solid white", color: "white",
            fontFamily:"IBM Plex Sans Condensed"}} type="submit" value="Join"/>
        </form>
        
        </div>
    )
}

export default Login
