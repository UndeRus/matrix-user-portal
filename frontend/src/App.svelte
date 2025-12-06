<script>
  let username = "";
  let password = "";
  let newPassword = "";
  let message = "";

  async function register() {
    const res = await fetch("/api/register", {
      method: "POST",
      headers: {"Content-Type": "application/json"},
      body: JSON.stringify({username, password})
    });
    message = await res.text();
  }

  async function changePassword() {
    const res = await fetch("/api/change_password", {
      method: "POST",
      headers: {"Content-Type": "application/json"},
      body: JSON.stringify({username, password, new_password: newPassword})
    });
    message = await res.text();
  }
</script>

<link rel="stylesheet" href="/bundle.css" />

<h1>Matrix Portal</h1>

<section>
  <h2>Register</h2>
  <input placeholder="Username" bind:value={username} />
  <input type="password" placeholder="Password" bind:value={password} />
  <button on:click={register}>Register</button>
</section>

<section>
  <h2>Change Password</h2>
  <input placeholder="Username" bind:value={username} />
  <input type="password" placeholder="Old Password" bind:value={password} />
  <input type="password" placeholder="New Password" bind:value={newPassword} />
  <button on:click={changePassword}>Change Password</button>
</section>

<p>{message}</p>

