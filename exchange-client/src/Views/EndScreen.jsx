import { useState, useEffect } from 'react'



function EndScreen({final_score}) {
    return (
        <div style={{width:"100%", height:"100%", display:"flex",
            flexDirection:"column", alignItems:"center", justifyContent:"center", position:"absolute"
        }}>
        <div style={{fontFamily:"IBM Plex Sans", color: "white", fontSize:"40px", marginBottom:"10px"}}>Final Score: {final_score}</div>
        </div>
    )
}

export default EndScreen
