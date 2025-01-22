let prices = [[0, 3, 4],
[13, 9, 4],
[26, 2, 3],
[39, 8, 3],
[52, 3, 4],
[65, 5, 1],
[78, 8, 3],
[91, 8, 5],
[104, 6, 4],
[117, 4, 5],
[130, 10, 0],
[143, 2, 3],
[156, 5, 2],
[169, 6, 1],
[182, 0, 5],
[195, 3, 4],
[208, 6, 5],
[221, 9, 4],
[234, 0, 1],
[247, 3, 5],
[260, 5, 1],
[273, 8, 4],
[286, 3, 0],
[299, 5, 0],
[312, 6, 5],
[325, 3, 4],
[338, 3, 2],
[351, 6, 4],
[364, 3, 4],
[598, 2, 0]]

/*
[377, 1, 0],
[390, 5, 3],
[403, 0, 3],
[416, 1, 0],
[429, 2, 3],
[442, 4, 5],
[455, 9, 5],
[468, 7, 2],
[481, 3, 5],
[494, 6, 1],
[507, 10, 2],
[520, 9, 3],
[533, 8, 3],
[546, 8, 1],
[559, 8, 4],
[572, 1, 3],
[585, 9, 2],
[598, 2, 0],
[611, 7, 2],
[624, 3, 2],
[637, 3, 2],
[650, 2, 2],
[663, 8, 3],
[676, 7, 4],
[689, 9, 4],
[702, 4, 4],
[715, 8, 3],
[728, 5, 5],
[741, 7, 5],
[754, 5, 1],
[767, 7, 4]]
*/


function makeCandles(prices, idx, candles) {
    //no new prices
    if (idx >= prices.length) return idx;  
    
    let open, hi, lo, close, volume;
    
    let end_time = Math.floor(prices.at(-1)[0] / 60);
    let t = Math.floor(prices[idx][0] / 60)

    while (t < end_time) {
        //create tick for time period t
        open = hi = lo = prices[idx][1]
        volume = prices[idx][2]
        while (Math.floor(prices[idx + 1][0] / 60) < t + 1) {
            hi = Math.max(hi, prices[idx][1])
            lo = Math.min(lo, prices[idx][1])
            volume += prices[idx][2]
            idx++
        }
        close = prices[idx][1]
        candles.push({ open, hi, lo, close, volume })
        idx++

        //fill in empty ticks until we get to next period with order
        let next = (prices[idx][0] / 60)
        while (t + 2 <= next) {
            candles.push({open: close, hi: close, lo: close, close: close, volume: 0})
            t += 1
        }

        //get start time of next period
        t = Math.floor(prices[idx][0] / 60)
    } return idx
}


let candles = [
    { open: 3, hi: 9, lo: 2, close: 3, volume: 18 },
    { open: 5, hi: 8, lo: 5, close: 4, volume: 14 },
    { open: 10, hi: 10, lo: 2, close: 6, volume: 5 },
    { open: 0, hi: 9, lo: 0, close: 0, volume: 23 },
    { open: 3, hi: 8, lo: 3, close: 5, volume: 15 },
    { open: 6, hi: 6, lo: 3, close: 6, volume: 16 }
] 
let idx = makeCandles(prices, 28, candles)

console.log(candles, idx)