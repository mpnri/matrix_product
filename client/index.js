const apiUrl = "http://localhost:8000/";

const submit = document.getElementById("submit");
let isLoading = false;
submit.addEventListener("click", () => {
  if (isLoading) return;
  let isError = false;

  const getItemVal = (name) =>
    document.getElementById(name)?.value === "" ||
    isNaN(+document.getElementById(name)?.value)
      ? (isError = true)
      : +document.getElementById(name)?.value;

  const data = {
    matrix_a: [
      [getItemVal("ma1"), getItemVal("ma2")],
      [getItemVal("ma3"), getItemVal("ma4")],
    ],
    matrix_b: [
      [getItemVal("mb1"), getItemVal("mb2")],
      [getItemVal("mb3"), getItemVal("mb4")],
    ],
  };
  if (isError) {
    alert("Invalid input");
    return;
  }
  isLoading = true;
  submit.innerHTML = `
    <div class="spinner-border text-light" role="status">
      <span class="sr-only">Loading...</span>
    </div>
  `;

  console.log(data);
  fetch(apiUrl, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(data),
  })
    .then((res) => {
      console.log(res);
      return res.text();
    })
    .then((res) => {
      console.log(res);
      alert(res);
    })
    .catch((err) => {
      console.log(err);
      alert("Error");
    })
    .finally(() => {
      isLoading = false;
      submit.innerHTML = `See Product`;
    });
});
