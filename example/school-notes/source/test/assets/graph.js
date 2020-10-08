function run(parent) {
    let canvas = document.createElement("canvas");
    parent.appendChild(canvas);
    let ctx = canvas.getContext("2d");
    ctx.moveTo(0, 0);
    ctx.lineTo(200, 100);
    ctx.stroke();
}