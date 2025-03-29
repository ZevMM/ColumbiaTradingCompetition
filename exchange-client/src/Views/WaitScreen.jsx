import { useState, useEffect } from 'react'



function WaitScreen() {
    return (
        <div style={{width:"100%", height:"100%", display:"flex",
            flexDirection:"column", alignItems:"center", justifyContent:"center", position:"absolute"
        }}>
        <div style={{fontFamily:"IBM Plex Sans", color: "white", fontSize:"40px", marginBottom:"10px"}}>The game will begin shortly</div>
        </div>
    )
}

export default WaitScreen