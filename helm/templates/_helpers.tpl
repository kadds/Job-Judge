{{- define "deployment" }}
apiVersion: apps/v1
kind: Deployment
metadata: 
  name: {{ .Chart.Name }}
  namespace: {{ .Release.Namespace }}
spec:
  selector:
    matchLabels:
      name: {{ .Chart.Name }}
  replicas: {{ .Values.service.replicas }}
  template: 
    metadata:
      labels:
        name: {{ .Chart.Name }}
    spec:
      imagePullSecrets:
        - name: {{ .Values.global.image.pullSecrets }}
      containers:
{{- end }}

{{- define "container" }}
- name: {{ .Chart.Name }}
  image: {{ .Values.global.image.prefix }}{{ .Chart.Name }}:{{ .Values.global.image.tag }}
  imagePullPolicy: {{ .Values.global.image.pullPolicy }}
  {{- with .Values.resources }}
  resources: 
      {{- toYaml . | nindent 4 }}
  {{- end }}
  ports:
    - containerPort: 11100
      name: {{ .Chart.Name }}
{{- end }}

{{- define "container.env"}}
envFrom:
  - configMapRef:
      name: {{ .Release.Name }}-cfg
env:
  - name: JJ_SERVICE_IP
    valueFrom:
      fieldRef:
        fieldPath: status.podIP
  - name: JJ_SERVICE_NAME
    valueFrom:
      fieldRef:
        fieldPath: metadata.name
  - name: JJ_SERVICE_MODULE
    value: {{ .Chart.Name }}
{{- end }}

{{- define "container.env_user" }}
- name: JJ_USERNAME
  valueFrom:
    secretKeyRef:
      key: username
      name: {{ .Release.Name }}-secret
- name: JJ_PASSWORD
  valueFrom:
    secretKeyRef:
      key: password
      name: {{ .Release.Name }}-secret
{{- end }}

{{- define "container.env_db" }}
- name: JJ_COMM_DATABASE_URL
  valueFrom:
    secretKeyRef:
      key: url
      name: {{ .Release.Name }}-db-secret
{{- end }}

{{- define "node_service" }}
apiVersion: v1
kind: Service
metadata:
  name: {{ .Chart.Name }}
  namespace: {{ .Release.Namespace }}
spec:
  type: NodePort
  selector: 
    name: {{ .Chart.Name }}
  ports:
    - targetPort: 11100
      nodePort: {{ .Values.service.nodePort }}
      port: 11100
      name: {{ .Chart.Name }}
{{- end }}

{{- define "service" }}
apiVersion: v1
kind: Service
metadata:
  name: {{ .Chart.Name }}
  namespace: {{ .Release.Namespace }}
spec:
  clusterIP: None
  selector: 
    name: {{ .Chart.Name }}
  ports:
    - targetPort: 11100
      port: 11100
      name: {{ .Chart.Name }}
{{- end }}

{{- define "stateful_service" }}
apiVersion: v1
kind: Service
metadata:
  name: {{ .Chart.Name }}
  namespace: {{ .Release.Namespace }}
spec:
  clusterIP: None
  selector: 
    name: {{ .Chart.Name }}
  ports:
    - targetPort: 11100
      port: 11100
      name: {{ .Chart.Name }}
{{- end }}

//* entry *//

{{- define "app.deployment" }}
{{- include "deployment" . }}
{{- include "container" . | indent 8 }}
{{- include "container.env" . | indent 10 }}
{{- end }}

{{- define "app.deployment_db" }}
{{- include "deployment" . }}
{{- include "container" . | indent 8 }}
{{- include "container.env" . | indent 10 }}
{{- include "container.env_db" . | indent 12 }}
{{- end }}

{{- define "app.deployment_user" }}
{{- include "deployment" . }}
{{- include "container" . | indent 8 }}
{{- include "container.env" . | indent 10 }}
{{- include "container.env_user" . | indent 12 }}
{{- end }}

{{- define "app.service" }}
{{- include "service" . }}
{{- end }}

{{- define "app.stateful_service" }}
{{- include "stateful_service" . }}
{{- end }}

{{- define "app.node_service" }}
{{- include "node_service" . }}
{{- end }}