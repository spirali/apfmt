fn f() {
    {
        {
            {
                let _ = mouse_click_tx.try_send(deno_runtime::ops::desktop::DesktopEvent::MouseClick { x, y, button, modifiers, aaaaaaa });
            }
        }
    }
}
